import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import test from "node:test";

const exec = promisify(execFile);

test("package metadata exposes only the Node 20 CLI release surface", async () => {
  const packageJson = JSON.parse(await readFile("package.json", "utf8"));
  assert.equal(packageJson.private, undefined);
  assert.equal(packageJson.bin.patchfleet, "bin/patchfleet.mjs");
  assert.equal(packageJson.engines.node, ">=20.9.0");
  assert.equal(packageJson.dependencies.next, "15.5.19");
  assert.equal(packageJson.scripts.prepack, "npm run build");
  assert.match(packageJson.scripts.build, /sanitize-package/);
  assert(packageJson.files.includes(".next"));
  assert.equal(packageJson.files.includes("tests"), false);
});

test("CLI help, stopped status, stale metadata cleanup, and empty recovery are safe", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-cli-"));
  const env = { ...process.env, PATCHFLEET_DATA_DIR: dataDir, PATCHFLEET_PORT: "39123" };
  assert.match((await exec(process.execPath, ["bin/patchfleet.mjs", "--help"], { env })).stdout, /start\|stop\|status\|recover/);
  assert.match((await exec(process.execPath, ["bin/patchfleet.mjs", "status"], { env })).stdout, /stopped/);

  await writeFile(join(dataDir, "runtime.json"), `${JSON.stringify({
    schemaVersion: 1,
    runtimeId: "runtime:00000000-0000-4000-8000-000000000000",
    pid: process.pid,
    port: 39123,
    startedAt: "2026-07-17T00:00:00.000Z",
    version: "0.1.0",
  })}\n`, { mode: 0o600 });
  assert.match((await exec(process.execPath, ["bin/patchfleet.mjs", "status"], { env })).stdout, /stopped/);
  await assert.rejects(() => readFile(join(dataDir, "runtime.json")), { code: "ENOENT" });
  assert.match((await exec(process.execPath, ["bin/patchfleet.mjs", "recover"], { env })).stdout, /recovered/);
});
