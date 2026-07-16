import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createServer } from "node:net";
import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const exec = promisify(execFile);
const directory = await mkdtemp(join(tmpdir(), "patchfleet-package-smoke-"));
const packDir = join(directory, "pack");
const prefix = join(directory, "prefix");
const dataDir = join(directory, "data");

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => server.once("error", reject).listen(0, "127.0.0.1", resolve));
  const selected = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return selected;
}

try {
  await import("node:fs/promises").then(({ mkdir }) => mkdir(packDir, { recursive: true }));
  const packed = JSON.parse((await exec("npm", [
    "pack", "--json", "--ignore-scripts", "--pack-destination", packDir,
  ], { maxBuffer: 10 * 1024 * 1024 })).stdout)[0];
  const names = packed.files.map((item) => item.path);
  assert(names.includes("bin/patchfleet.mjs"));
  assert(names.includes("docs/install.md"));
  assert(names.includes(".next/BUILD_ID"));
  assert(names.includes("extensions/patchfleet-gemini/hook.mjs"));
  assert.equal(names.some((name) => name.startsWith("tests/") || name.startsWith(".git/") || name.includes(".env")), false);

  const tarball = join(packDir, packed.filename);
  await exec("npm", ["install", "--global", "--prefix", prefix, tarball, "--ignore-scripts"], {
    maxBuffer: 10 * 1024 * 1024,
  });
  const packageRoot = process.platform === "win32"
    ? join(prefix, "node_modules", "patchfleet")
    : join(prefix, "lib", "node_modules", "patchfleet");
  const cli = process.platform === "win32" ? join(prefix, "patchfleet.cmd") : join(prefix, "bin", "patchfleet");
  const serverManifest = JSON.parse(await readFile(join(packageRoot, ".next", "required-server-files.json"), "utf8"));
  assert.equal(serverManifest.appDir, ".");
  assert.equal(serverManifest.config.outputFileTracingRoot, ".");
  assert.equal(serverManifest.config.turbopack.root, ".");
  const selectedPort = await freePort();
  const env = {
    ...process.env,
    PATCHFLEET_DATA_DIR: dataDir,
    PATCHFLEET_PORT: String(selectedPort),
  };
  const invoke = (argument, options = {}) => exec(cli, [argument], {
    env,
    shell: process.platform === "win32",
    ...options,
  });
  if (process.platform !== "win32") assert.notEqual((await stat(cli)).mode & 0o111, 0);
  assert.match((await invoke("start", { timeout: 30_000 })).stdout, /started/);
  assert.match((await invoke("status")).stdout, /running/);
  if (process.platform !== "win32") {
    assert.equal((await stat(dataDir)).mode & 0o777, 0o700);
    assert.equal((await stat(join(dataDir, "runtime.json"))).mode & 0o777, 0o600);
    assert.equal((await stat(join(dataDir, "patchfleet.log"))).mode & 0o777, 0o600);
  }
  assert.match((await invoke("stop", { timeout: 20_000 })).stdout, /stopped/);
  assert.match((await invoke("recover")).stdout, /recovered/);
  assert.equal((await readFile(join(packageRoot, "package.json"), "utf8")).includes('"private"'), false);
  process.stdout.write("package smoke: pack -> clean install -> start/status/stop/recover PASS\n");
} catch (error) {
  const log = await readFile(join(dataDir, "patchfleet.log"), "utf8").catch(() => "no Patchfleet log");
  throw new Error(`${error.message}\n${log}`, { cause: error });
} finally {
  await rm(directory, { recursive: true, force: true });
}
