import { randomUUID } from "node:crypto";
import { chmod, lstat, mkdir, open, readFile, readdir, rename, unlink } from "node:fs/promises";
import { join } from "node:path";
import { validateProviderLifecycleSignal } from "../domain/provider-lifecycle-signal.mjs";

const INBOX = "gemini-inbox";
const SIGNAL_PREFIX = /^signal-/;
const FILE = /^signal-\d{13}-[0-9a-f-]{36}\.json$/i;
const MAX_SIGNAL_BYTES = 4_096;

export function resolveMoxcrewDataDirectory(override, sourceRoot = process.cwd()) {
  return override ?? process.env.MOXCREW_DATA_DIR ?? process.env.PATCHFLEET_DATA_DIR ?? join(sourceRoot, ".patchfleet");
}

function inboxDirectory(dataDir, sourceRoot) {
  return join(resolveMoxcrewDataDirectory(dataDir, sourceRoot), INBOX);
}

export async function writeGeminiInboxSignal(value, { dataDir, sourceRoot } = {}) {
  const signal = validateProviderLifecycleSignal(value);
  if (signal.providerId !== "gemini") throw new TypeError("invalid Gemini lifecycle signal");

  const directory = inboxDirectory(dataDir, sourceRoot);
  await mkdir(directory, { recursive: true, mode: 0o700 });
  await chmod(directory, 0o700);
  const id = randomUUID();
  const temporary = join(directory, `.signal-${id}.tmp`);
  const target = join(directory, `signal-${Date.parse(signal.observedAt)}-${id}.json`);
  const handle = await open(temporary, "wx", 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(signal)}\n`, "utf8");
    await handle.sync();
  } finally {
    await handle.close();
  }
  await rename(temporary, target);
  return target;
}

export async function drainGeminiInbox(persistSignal, { dataDir } = {}) {
  const directory = inboxDirectory(dataDir);
  let names;
  try {
    names = (await readdir(directory)).filter((name) => SIGNAL_PREFIX.test(name)).sort();
  } catch (error) {
    if (error.code === "ENOENT") return undefined;
    throw error;
  }

  let result;
  for (const name of names) {
    const path = join(directory, name);
    let signal;
    try {
      if (!FILE.test(name)) throw new TypeError();
      const metadata = await lstat(path);
      if (!metadata.isFile() || metadata.size > MAX_SIGNAL_BYTES) throw new TypeError();
      signal = validateProviderLifecycleSignal(JSON.parse(await readFile(path, "utf8")));
      if (signal.providerId !== "gemini") throw new TypeError();
    } catch {
      await unlink(path).catch(() => undefined);
      continue;
    }

    try {
      result = await persistSignal(signal);
      await unlink(path);
    } catch {
      // Durable append or cleanup can be retried on the next manual refresh.
    }
  }
  return result;
}
