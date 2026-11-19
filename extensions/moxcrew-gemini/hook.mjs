import { decodeGeminiHook } from "../../lib/providers/gemini.mjs";
import { writeGeminiInboxSignal } from "../../lib/runtime/gemini-inbox.mjs";
import { realpath, lstat } from "node:fs/promises";
import { homedir, platform } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const MAX_BYTES = 16_384;

async function dataDirectory() {
  const configured = process.env.MOXCREW_DATA_DIR;
  const previous = process.env.PATCHFLEET_DATA_DIR;
  if (configured && previous && configured !== previous) throw new TypeError();
  if (configured || previous) {
    const chosen = configured ?? previous;
    if (!isAbsolute(chosen)) throw new TypeError();
    return chosen;
  }
  const base = platform() === "win32"
    ? process.env.APPDATA
    : platform() === "darwin"
      ? join(homedir(), "Library", "Application Support")
      : (process.env.XDG_DATA_HOME && isAbsolute(process.env.XDG_DATA_HOME)
          ? process.env.XDG_DATA_HOME : join(homedir(), ".local", "share"));
  if (!base) throw new TypeError();
  const current = join(base, platform() === "linux" ? "moxcrew" : "Moxcrew");
  const old = [join(base, "com.patchfleet.desktop")];
  if (platform() !== "linux") old.push(join(base, "Patchfleet"));
  const exists = async path => {
    try { return (await lstat(path)).isDirectory(); }
    catch (error) { if (error.code === "ENOENT") return false; throw error; }
  };
  const found = (await Promise.all(old.map(exists))).flatMap((yes, index) => yes ? [old[index]] : []);
  if (found.length > 1 || (found.length && await exists(current))) throw new TypeError();
  return found[0] ?? current;
}

async function readInput() {
  let input = "";
  for await (const chunk of process.stdin) {
    input += chunk;
    if (Buffer.byteLength(input) > MAX_BYTES) throw new TypeError();
  }
  return input;
}

try {
  const signal = decodeGeminiHook(await readInput());
  if (signal) {
    const sourceRoot = resolve(dirname(await realpath(fileURLToPath(import.meta.url))), "../..");
    await writeGeminiInboxSignal(signal, {
      dataDir: await dataDirectory(),
      sourceRoot,
    });
  }
} catch {
  // Observation is fail-open and never exposes the native hook payload.
}
process.stdout.write("{}");
