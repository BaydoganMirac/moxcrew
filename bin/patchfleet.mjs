#!/usr/bin/env node

import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { chmod, mkdir, open, readFile, rename, rm } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dataDir = process.env.PATCHFLEET_DATA_DIR ?? join(homedir(), ".patchfleet");
const runtimeFile = join(dataDir, "runtime.json");
const logFile = join(dataDir, "patchfleet.log");
const command = process.argv[2] ?? "help";

function port() {
  const value = Number(process.env.PATCHFLEET_PORT ?? process.env.PORT ?? "3000");
  if (!Number.isSafeInteger(value) || value < 1 || value > 65535) throw new TypeError("invalid Patchfleet port");
  return value;
}

function validateRuntime(value) {
  const fields = ["schemaVersion", "runtimeId", "pid", "port", "startedAt", "version"];
  if (
    !value || typeof value !== "object" || Array.isArray(value) ||
    Object.keys(value).length !== fields.length || Object.keys(value).some((key) => !fields.includes(key)) ||
    value.schemaVersion !== 1 || typeof value.runtimeId !== "string" || !/^runtime:[0-9a-f-]{36}$/i.test(value.runtimeId) ||
    !Number.isSafeInteger(value.pid) || value.pid < 1 || !Number.isSafeInteger(value.port) || value.port < 1 || value.port > 65535 ||
    typeof value.startedAt !== "string" || new Date(value.startedAt).toISOString() !== value.startedAt ||
    typeof value.version !== "string" || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value.version)
  ) throw new TypeError("invalid runtime metadata");
  return value;
}

async function runtime() {
  try {
    return validateRuntime(JSON.parse(await readFile(runtimeFile, "utf8")));
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

async function writeRuntime(value) {
  await mkdir(dataDir, { recursive: true, mode: 0o700 });
  await chmod(dataDir, 0o700);
  const temporary = `${runtimeFile}.${randomUUID()}.tmp`;
  const handle = await open(temporary, "wx", 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(validateRuntime(value))}\n`, "utf8");
    await handle.sync();
  } finally {
    await handle.close();
  }
  await rename(temporary, runtimeFile);
  await chmod(runtimeFile, 0o600);
}

function processExists(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code === "EPERM";
  }
}

async function health(checkPort) {
  try {
    const response = await fetch(`http://127.0.0.1:${checkPort}/api/health`, {
      cache: "no-store",
      signal: AbortSignal.timeout(750),
    });
    let body;
    try {
      body = await response.json();
    } catch {
      body = null;
    }
    return {
      reachable: true,
      runtimeId: response.ok && body?.schemaVersion === 1 && typeof body.runtimeId === "string"
        ? body.runtimeId
        : null,
    };
  } catch {
    return { reachable: false, runtimeId: null };
  }
}

async function running(value) {
  if (!value || !processExists(value.pid)) return false;
  return (await health(value.port)).runtimeId === value.runtimeId;
}

async function start() {
  const previous = await runtime();
  if (await running(previous)) {
    process.stdout.write(`Patchfleet is already running at http://127.0.0.1:${previous.port}\n`);
    return;
  }
  if (previous) await rm(runtimeFile, { force: true });

  const selectedPort = port();
  if ((await health(selectedPort)).reachable) throw new Error(`port ${selectedPort} is already in use`);
  const packageJson = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
  const state = {
    schemaVersion: 1,
    runtimeId: `runtime:${randomUUID()}`,
    pid: 1,
    port: selectedPort,
    startedAt: new Date().toISOString(),
    version: packageJson.version,
  };

  await mkdir(dataDir, { recursive: true, mode: 0o700 });
  await chmod(dataDir, 0o700);
  const log = await open(logFile, "a", 0o600);
  await chmod(logFile, 0o600);
  let child;
  try {
    child = spawn(process.execPath, [join(root, "scripts", "local-next.mjs"), "start"], {
      cwd: root,
      detached: true,
      env: {
        ...process.env,
        PATCHFLEET_DATA_DIR: dataDir,
        PATCHFLEET_RUNTIME_ID: state.runtimeId,
        PORT: String(selectedPort),
      },
      stdio: ["ignore", log.fd, log.fd],
      windowsHide: true,
    });
    if (!Number.isSafeInteger(child.pid) || child.pid < 1) throw new Error("Patchfleet process did not start");
    child.on("error", () => undefined);
    state.pid = child.pid;
    await writeRuntime(state);
  } catch (error) {
    if (child?.pid && processExists(child.pid)) process.kill(child.pid, "SIGTERM");
    throw error;
  } finally {
    await log.close();
  }
  child.unref();

  for (let attempt = 0; attempt < 150; attempt += 1) {
    if ((await health(selectedPort)).runtimeId === state.runtimeId) {
      process.stdout.write(`Patchfleet started at http://127.0.0.1:${selectedPort}\n`);
      return;
    }
    if (!processExists(state.pid)) break;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
  }
  if (processExists(state.pid)) process.kill(state.pid, "SIGTERM");
  await rm(runtimeFile, { force: true });
  throw new Error(`Patchfleet did not start; inspect ${logFile}`);
}

async function status() {
  const state = await runtime();
  if (await running(state)) {
    process.stdout.write(`Patchfleet is running at http://127.0.0.1:${state.port} (pid ${state.pid})\n`);
    return;
  }
  if (state) await rm(runtimeFile, { force: true });
  process.stdout.write("Patchfleet is stopped.\n");
}

async function stop() {
  const state = await runtime();
  if (!(await running(state))) {
    if (state) await rm(runtimeFile, { force: true });
    process.stdout.write("Patchfleet is already stopped.\n");
    return;
  }
  process.kill(state.pid, "SIGTERM");
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (!(await running(state))) {
      await rm(runtimeFile, { force: true });
      process.stdout.write("Patchfleet stopped.\n");
      return;
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100));
  }
  throw new Error("Patchfleet did not stop cleanly");
}

async function recover() {
  const state = await runtime();
  if (await running(state)) throw new Error("stop Patchfleet before recovery");
  if (state) await rm(runtimeFile, { force: true });
  const [{ rebuildProjection }, { readCloudState }] = await Promise.all([
    import(pathToFileURL(join(root, "lib", "runtime", "observation-store.mjs"))),
    import(pathToFileURL(join(root, "lib", "cloud", "connection.mjs"))),
  ]);
  await rebuildProjection({ dataDir });
  await readCloudState({ dataDir });
  process.stdout.write("Patchfleet projections recovered from the durable event log.\n");
}

function help() {
  process.stdout.write("Usage: patchfleet <start|stop|status|recover>\n");
}

try {
  if (command === "start") await start();
  else if (command === "stop") await stop();
  else if (command === "status") await status();
  else if (command === "recover") await recover();
  else if (command === "help" || command === "--help" || command === "-h") help();
  else throw new TypeError("unknown Patchfleet command");
} catch (error) {
  process.stderr.write(`Patchfleet: ${error.message}\n`);
  process.exitCode = 1;
}
