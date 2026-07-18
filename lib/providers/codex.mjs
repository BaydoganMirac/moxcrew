import { once } from "node:events";
import { createInterface } from "node:readline";
import { normalizeObservation, safeObservationError } from "../domain/observation.mjs";
import { execCli, spawnCli } from "./cli-process.mjs";

const DEFAULT_TIMEOUT_MS = 5_000;
const INTERACTIVE_SOURCES = ["cli", "vscode", "exec", "appServer"];
const MINIMUM_CONTROL_VERSION = [0, 144, 1];

export class CodexAdapterFailure extends Error {
  constructor(code) {
    super(code);
    this.code = code;
  }
}

export function supportsCodexControl(observation) {
  const provider = observation?.provider;
  const match = /^(\d+)\.(\d+)\.(\d+)(?:\+[A-Za-z0-9.-]+)?$/.exec(provider?.version ?? "");
  if (
    provider?.id !== "codex" ||
    provider.state !== "available" ||
    provider.capabilities?.recentObservation !== true ||
    provider.capabilities?.explicitLiveStatus !== true ||
    !match
  ) {
    return false;
  }
  const version = match.slice(1).map(Number);
  if (version.some((part) => !Number.isSafeInteger(part))) return false;
  for (let index = 0; index < version.length; index += 1) {
    if (version[index] !== MINIMUM_CONTROL_VERSION[index]) {
      return version[index] > MINIMUM_CONTROL_VERSION[index];
    }
  }
  return true;
}

function unavailable(code, observedAt, version = null) {
  return normalizeObservation({
    provider: {
      id: "codex",
      displayName: "Codex",
      state: code === "CODEX_NOT_FOUND" ? "unavailable" : "degraded",
      version,
      capabilities: { recentObservation: false, explicitLiveStatus: false },
      error: safeObservationError("codex", code),
    },
    observedAt,
    sessions: [],
  });
}

export function probeCodex({ command = "codex", timeoutMs = DEFAULT_TIMEOUT_MS } = {}) {
  return new Promise((resolve) => {
    execCli(
      command,
      ["--version"],
      { encoding: "utf8", maxBuffer: 1_024, timeout: timeoutMs, windowsHide: true },
      (error, stdout) => {
        if (error?.code === "ENOENT") {
          return resolve({ state: "unavailable", code: "CODEX_NOT_FOUND", version: null });
        }
        if (error?.killed || error?.signal === "SIGTERM") {
          return resolve({ state: "degraded", code: "CODEX_PROBE_TIMEOUT", version: null });
        }
        if (error) {
          return resolve({ state: "degraded", code: "CODEX_PROBE_FAILED", version: null });
        }

        const match = /^codex-cli (\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?)\s*$/.exec(stdout);
        if (!match) {
          return resolve({ state: "degraded", code: "CODEX_VERSION_MALFORMED", version: null });
        }
        return resolve({ state: "available", version: match[1] });
      },
    );
  });
}

export function createCodexAppServerClient(command, timeoutMs = DEFAULT_TIMEOUT_MS) {
  const child = spawnCli(command, ["app-server", "--stdio"], {
    stdio: ["pipe", "pipe", "ignore"],
    windowsHide: true,
  });
  const lines = createInterface({ input: child.stdout, crlfDelay: Infinity });
  const pending = new Map();
  let nextId = 1;
  let failure;

  function fail(code) {
    if (failure) return;
    failure = new CodexAdapterFailure(code);
    for (const { reject, timer } of pending.values()) {
      clearTimeout(timer);
      reject(failure);
    }
    pending.clear();
  }

  child.once("error", () => fail("CODEX_APP_SERVER_START_FAILED"));
  child.once("exit", () => fail("CODEX_APP_SERVER_EXITED"));
  child.stdin.on("error", () => fail("CODEX_APP_SERVER_EXITED"));
  lines.on("line", (line) => {
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      fail("CODEX_PROTOCOL_INVALID_JSON");
      return;
    }
    if (!message || typeof message !== "object" || Array.isArray(message)) {
      fail("CODEX_PROTOCOL_MALFORMED");
      return;
    }
    if (typeof message.method === "string") {
      if (Object.hasOwn(message, "id")) {
        child.stdin.write(`${JSON.stringify({
          id: message.id,
          error: { code: -32601, message: "Method not found" },
        })}\n`);
      }
      return;
    }
    if (!pending.has(message.id)) {
      fail("CODEX_PROTOCOL_MALFORMED");
      return;
    }

    const request = pending.get(message.id);
    pending.delete(message.id);
    clearTimeout(request.timer);
    if (Object.hasOwn(message, "error")) {
      request.reject(new CodexAdapterFailure("CODEX_PROTOCOL_METHOD_ERROR"));
    } else if (!Object.hasOwn(message, "result")) {
      request.reject(new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED"));
    } else {
      request.resolve(message.result);
    }
  });

  function request(method, params) {
    if (failure) return Promise.reject(failure);
    const id = nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new CodexAdapterFailure("CODEX_APP_SERVER_TIMEOUT"));
      }, timeoutMs);
      timer.unref();
      pending.set(id, { resolve, reject, timer });
      child.stdin.write(`${JSON.stringify({ method, id, params })}\n`, (error) => {
        if (error && pending.has(id)) {
          clearTimeout(timer);
          pending.delete(id);
          reject(new CodexAdapterFailure("CODEX_APP_SERVER_EXITED"));
        }
      });
    });
  }

  function notify(method) {
    if (failure) throw failure;
    child.stdin.write(`${JSON.stringify({ method })}\n`);
  }

  async function close() {
    lines.close();
    if (child.exitCode !== null || child.signalCode !== null) return;
    const closed = once(child, "close").catch(() => undefined);
    child.stdin.end();
    child.kill("SIGTERM");
    const timer = new Promise((resolve) => {
      const id = setTimeout(resolve, 500);
      id.unref();
    });
    await Promise.race([closed, timer]);
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  }

  return { request, notify, close };
}

function timestamp(seconds, optional = false) {
  if (optional && (seconds === null || seconds === undefined)) return undefined;
  if (!Number.isSafeInteger(seconds) || seconds < 0) {
    throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
  }
  const value = new Date(seconds * 1_000);
  if (Number.isNaN(value.valueOf())) throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
  return value.toISOString();
}

function normalizeThread(thread, observedAt) {
  if (!thread || typeof thread !== "object" || Array.isArray(thread)) {
    throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
  }
  if (!/^[A-Za-z0-9._:-]{1,256}$/.test(thread.id) || !Array.isArray(thread.turns)) {
    throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
  }
  const threadStatus = thread.status?.type;
  if (typeof threadStatus !== "string") throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");

  const latest = thread.turns.at(-1);
  if (latest !== undefined && (!latest || typeof latest !== "object" || typeof latest.status !== "string")) {
    throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
  }

  let status = "unknown";
  if (["completed", "failed", "interrupted"].includes(latest?.status)) status = latest.status;
  else if (threadStatus === "active" || latest?.status === "inProgress") status = "running";

  const terminalAt = ["completed", "failed", "interrupted"].includes(status)
    ? timestamp(latest?.completedAt, true)
    : undefined;

  return {
    session: {
      providerSessionId: thread.id,
      status,
      createdAt: timestamp(thread.createdAt),
      lastObservedAt: observedAt,
      ...(terminalAt ? { terminalAt } : {}),
    },
    systemError: threadStatus === "systemError",
  };
}

export async function observeCodex({
  command = "codex",
  timeoutMs = DEFAULT_TIMEOUT_MS,
  now = () => new Date(),
} = {}) {
  const observedAt = now().toISOString();
  const probe = await probeCodex({ command, timeoutMs });
  if (probe.state !== "available") return unavailable(probe.code, observedAt);

  const client = createCodexAppServerClient(command, timeoutMs);
  try {
    const initialized = await client.request("initialize", {
      clientInfo: { name: "patchfleet", title: "Patchfleet", version: "0.1.0" },
      capabilities: null,
    });
    if (!initialized || typeof initialized !== "object" || Array.isArray(initialized)) {
      throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
    }
    client.notify("initialized");

    const listed = await client.request("thread/list", {
      archived: false,
      limit: 20,
      sortDirection: "desc",
      sortKey: "recency_at",
      sourceKinds: INTERACTIVE_SOURCES,
    });
    if (!listed || typeof listed !== "object" || !Array.isArray(listed.data)) {
      throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
    }

    const ids = listed.data.slice(0, 20).map((thread) => {
      if (!thread || typeof thread !== "object" || !/^[A-Za-z0-9._:-]{1,256}$/.test(thread.id)) {
        throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
      }
      return thread.id;
    });
    if (new Set(ids).size !== ids.length) throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");

    const normalized = [];
    let systemError = false;
    for (const threadId of ids) {
      const result = await client.request("thread/read", { threadId, includeTurns: true });
      if (
        !result ||
        typeof result !== "object" ||
        !Object.hasOwn(result, "thread") ||
        result.thread?.id !== threadId
      ) {
        throw new CodexAdapterFailure("CODEX_PROTOCOL_MALFORMED");
      }
      const item = normalizeThread(result.thread, observedAt);
      normalized.push(item.session);
      systemError ||= item.systemError;
    }

    return normalizeObservation({
      provider: {
        id: "codex",
        displayName: "Codex",
        state: systemError ? "degraded" : "available",
        version: probe.version,
        capabilities: { recentObservation: true, explicitLiveStatus: true },
        ...(systemError ? { error: safeObservationError("codex", "CODEX_SYSTEM_ERROR") } : {}),
      },
      observedAt,
      sessions: normalized,
    });
  } catch (error) {
    const code = error instanceof CodexAdapterFailure ? error.code : "CODEX_PROTOCOL_MALFORMED";
    return unavailable(code, observedAt, probe.version);
  } finally {
    await client.close();
  }
}
