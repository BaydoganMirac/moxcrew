import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { join } from "node:path";

const mode = process.argv[2];
if (!new Set(["dev", "start"]).has(mode)) throw new TypeError("expected dev or start");

const port = Number(process.env.PORT ?? "3000");
if (!Number.isSafeInteger(port) || port < 1 || port > 65535) throw new TypeError("invalid PORT");
const syncToken = randomUUID();
const dataDir = process.env.PATCHFLEET_DATA_DIR ?? join(homedir(), ".patchfleet");

const require = createRequire(import.meta.url);
const child = spawn(process.execPath, [
  require.resolve("next/dist/bin/next"),
  mode,
  ...(mode === "dev" ? ["--turbopack"] : []),
  "--hostname",
  "127.0.0.1",
  "--port",
  String(port),
], {
  env: {
    ...process.env,
    PATCHFLEET_DATA_DIR: dataDir,
    PATCHFLEET_OWNER_EPOCH: randomUUID(),
    PATCHFLEET_SYNC_TOKEN: syncToken,
  },
  stdio: "inherit",
});

const poll = setInterval(() => {
  void fetch(`http://127.0.0.1:${port}/api/cloud/sync`, {
    method: "POST",
    headers: { authorization: `Bearer ${syncToken}` },
    signal: AbortSignal.timeout(4_000),
  }).catch(() => undefined);
}, 5_000);

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.once(signal, () => child.kill(signal));
}
child.once("error", (error) => {
  throw error;
});
child.once("exit", (code, signal) => {
  clearInterval(poll);
  if (signal) process.kill(process.pid, signal);
  else process.exitCode = code ?? 1;
});
