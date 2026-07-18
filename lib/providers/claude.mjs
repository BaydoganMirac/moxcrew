import { execCli, terminateCli } from "./cli-process.mjs";

const DEFAULT_TIMEOUT_MS = 5_000;
const VERSION_BUFFER = 1_024;
const SNAPSHOT_BUFFER = 256 * 1_024;
const ID = /^[A-Za-z0-9._:-]+$/;
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;
const MIN_AGENT_VIEW_VERSION = [2, 1, 169];
const TERMINAL = new Set(["completed", "failed", "interrupted"]);
const ERRORS = Object.freeze({
  CLAUDE_NOT_FOUND: "Claude Code CLI is not installed or is not on PATH.",
  CLAUDE_PROBE_FAILED: "Claude Code CLI could not be checked.",
  CLAUDE_PROBE_TIMEOUT: "Claude Code CLI version check timed out.",
  CLAUDE_SNAPSHOT_FAILED: "Claude Code Agent View could not be checked.",
  CLAUDE_SNAPSHOT_MALFORMED: "Claude Code Agent View returned an unsupported response.",
  CLAUDE_SNAPSHOT_TIMEOUT: "Claude Code Agent View timed out.",
  CLAUDE_SNAPSHOT_UNSUPPORTED: "Claude Code Agent View returned an unsupported session shape.",
  CLAUDE_SURFACE_UNSUPPORTED: "This Claude Code version lacks the supported Agent View surface.",
  CLAUDE_VERSION_MALFORMED: "Claude Code CLI returned an unsupported version.",
});

function run(command, args, timeoutMs, maxBuffer) {
  return new Promise((resolve) => {
    let timedOut = false;
    let killTimer;
    const child = execCli(
      command,
      args,
      { encoding: "utf8", maxBuffer, windowsHide: true },
      (error, stdout) => {
        clearTimeout(timer);
        resolve({ error, stdout: error || timedOut ? "" : stdout, timedOut });
      },
    );
    const timer = setTimeout(() => {
      timedOut = true;
      terminateCli(child);
      killTimer = setTimeout(() => terminateCli(child, "SIGKILL"), 250);
      killTimer.unref();
    }, timeoutMs);
    timer.unref();
    child.once("close", () => clearTimeout(killTimer));
  });
}

function supportsAgentView(version) {
  const current = version.split(/[+-]/, 1)[0].split(".").map(Number);
  for (let index = 0; index < current.length; index++) {
    if (current[index] !== MIN_AGENT_VIEW_VERSION[index]) {
      return current[index] > MIN_AGENT_VIEW_VERSION[index];
    }
  }
  return true;
}

export async function probeClaude({ command = "claude", timeoutMs = DEFAULT_TIMEOUT_MS } = {}) {
  const { error, stdout, timedOut } = await run(command, ["--version"], timeoutMs, VERSION_BUFFER);
  if (error?.code === "ENOENT") return { state: "unavailable", code: "CLAUDE_NOT_FOUND", version: null };
  if (timedOut) return { state: "degraded", code: "CLAUDE_PROBE_TIMEOUT", version: null };
  if (error) return { state: "degraded", code: "CLAUDE_PROBE_FAILED", version: null };

  const match = /^(\S+) \(Claude Code\)\s*$/.exec(stdout);
  if (!match || !SEMVER.test(match[1])) {
    return { state: "degraded", code: "CLAUDE_VERSION_MALFORMED", version: null };
  }
  if (!supportsAgentView(match[1])) {
    return { state: "degraded", code: "CLAUDE_SURFACE_UNSUPPORTED", version: match[1] };
  }
  return { state: "available", version: match[1] };
}

function failed(code, observedAt, version = null, state = "degraded") {
  return {
    schemaVersion: 1,
    provider: {
      id: "claude",
      displayName: "Claude Code",
      state,
      version,
      capabilities: { recentObservation: false, explicitLiveStatus: false },
      error: { code, message: ERRORS[code] },
    },
    observedAt,
    sessions: [],
  };
}

function iso(value) {
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value) || value < 1_000_000_000_000 || value > 9_999_999_999_999) {
      return null;
    }
    return new Date(value).toISOString();
  }
  if (typeof value !== "string" || Number.isNaN(Date.parse(value))) return null;
  const normalized = new Date(value).toISOString();
  return normalized === value ? normalized : null;
}

function sessionId(entry) {
  const prefix = Object.hasOwn(entry, "id") ? "job:" : "session:";
  const native = Object.hasOwn(entry, "id") ? entry.id : entry.sessionId;
  if (typeof native !== "string" || native.length === 0) return null;
  const value = `${prefix}${native}`;
  return value.length <= 256 && ID.test(value) ? value : null;
}

function status(entry) {
  const native = Object.hasOwn(entry, "id") ? entry.state : entry.status;
  const mapping = Object.hasOwn(entry, "id")
    ? { working: "running", done: "completed", failed: "failed", stopped: "interrupted", blocked: "unknown" }
    : { running: "running", waiting: "unknown", idle: "unknown" };
  return typeof native === "string" ? mapping[native] : undefined;
}

function normalizeSessions(value, observedAt) {
  if (!Array.isArray(value)) return null;
  const sessions = [];
  const ids = new Set();
  for (const entry of value) {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) return null;
    const providerSessionId = sessionId(entry);
    const createdAt = iso(entry.startedAt);
    const normalizedStatus = status(entry);
    if (!providerSessionId || !createdAt || !normalizedStatus || ids.has(providerSessionId)) return null;
    ids.add(providerSessionId);
    sessions.push({ providerSessionId, status: normalizedStatus, createdAt, lastObservedAt: observedAt });
  }
  sessions.sort((left, right) =>
    Number(TERMINAL.has(left.status)) - Number(TERMINAL.has(right.status)) ||
    right.createdAt.localeCompare(left.createdAt) ||
    (left.providerSessionId < right.providerSessionId ? -1 : 1),
  );
  return sessions.slice(0, 20);
}

export async function observeClaude({
  command = "claude",
  timeoutMs = DEFAULT_TIMEOUT_MS,
  now = () => new Date(),
} = {}) {
  const observedAt = now().toISOString();
  const probe = await probeClaude({ command, timeoutMs });
  if (probe.state !== "available") {
    return failed(probe.code, observedAt, probe.version, probe.state);
  }

  const { error, stdout, timedOut } = await run(command, ["agents", "--json", "--all"], timeoutMs, SNAPSHOT_BUFFER);
  if (timedOut) return failed("CLAUDE_SNAPSHOT_TIMEOUT", observedAt, probe.version);
  if (error) return failed("CLAUDE_SNAPSHOT_FAILED", observedAt, probe.version);

  let native;
  try {
    native = JSON.parse(stdout);
  } catch {
    return failed("CLAUDE_SNAPSHOT_MALFORMED", observedAt, probe.version);
  }
  const sessions = normalizeSessions(native, observedAt);
  if (!sessions) return failed("CLAUDE_SNAPSHOT_UNSUPPORTED", observedAt, probe.version);

  return {
    schemaVersion: 1,
    provider: {
      id: "claude",
      displayName: "Claude Code",
      state: "available",
      version: probe.version,
      capabilities: { recentObservation: true, explicitLiveStatus: true },
    },
    observedAt,
    sessions,
  };
}
