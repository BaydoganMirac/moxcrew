import { stat, realpath } from "node:fs/promises";
import { homedir } from "node:os";
import { parse, join } from "node:path";
import {
  createCodexAppServerClient,
} from "./codex.mjs";

const OPAQUE_ID = /^[A-Za-z0-9._:-]{1,256}$/;
const SAFE_THREAD_SOURCE = /^patchfleet:[A-Za-z0-9._:-]{1,245}$/;
const controlKey = Symbol.for("patchfleet.codex-control");
const state = (globalThis[controlKey] ??= { client: null, command: null, opening: null, threads: new Set() });
state.threads ??= new Set();

export class CodexControlFailure extends Error {
  constructor(code, { outcomeUnknown = false } = {}) {
    super(code);
    this.code = code;
    this.outcomeUnknown = outcomeUnknown;
  }
}

function failure(code, options) {
  return new CodexControlFailure(code, options);
}

function record(value) {
  return value && typeof value === "object" && !Array.isArray(value) ? value : null;
}

function opaque(value) {
  if (typeof value !== "string" || !OPAQUE_ID.test(value)) {
    throw failure("CODEX_PROTOCOL_MALFORMED");
  }
  return value;
}

function turn(value) {
  const candidate = record(value);
  if (!candidate || !["inProgress", "completed", "failed", "interrupted"].includes(candidate.status)) {
    throw failure("CODEX_PROTOCOL_MALFORMED");
  }
  if (
    candidate.completedAt !== null &&
    candidate.completedAt !== undefined &&
    (!Number.isSafeInteger(candidate.completedAt) || candidate.completedAt < 0)
  ) {
    throw failure("CODEX_PROTOCOL_MALFORMED");
  }
  const terminalAt = candidate.completedAt === null || candidate.completedAt === undefined
    ? null
    : new Date(candidate.completedAt * 1_000).toISOString();
  return {
    providerTurnId: opaque(candidate.id),
    status: candidate.status === "inProgress" ? "running" : candidate.status,
    terminalAt,
  };
}

async function closeState() {
  const client = state.client;
  state.client = null;
  state.command = null;
  state.threads.clear();
  if (client) await client.close();
}

export async function closeCodexControl() {
  await closeState();
  state.opening = null;
}

async function client(command, timeoutMs) {
  if (state.client && state.command === command) return state.client;
  if (state.opening) return state.opening;
  state.opening = (async () => {
    await closeState();
    const next = createCodexAppServerClient(command, timeoutMs);
    try {
      const initialized = await next.request("initialize", {
        clientInfo: { name: "patchfleet", title: "Moxcrew", version: "0.1.0" },
        capabilities: null,
      });
      if (!record(initialized)) throw failure("CODEX_PROTOCOL_MALFORMED");
      next.notify("initialized");
      state.client = next;
      state.command = command;
      return next;
    } catch (error) {
      await next.close();
      throw error;
    } finally {
      state.opening = null;
    }
  })();
  return state.opening;
}

async function request(active, method, params, outcomeUnknown = false, validate = (value) => value) {
  try {
    return validate(await active.request(method, params));
  } catch (error) {
    const result = outcomeUnknown && error?.code !== "CODEX_PROTOCOL_METHOD_ERROR"
      ? failure("CODEX_OUTCOME_UNKNOWN", { outcomeUnknown: true })
      : failure(error?.code === "CODEX_PROTOCOL_METHOD_ERROR"
      ? `CODEX_${method.replaceAll("/", "_").toUpperCase()}_REJECTED`
      : "CODEX_CONTROL_FAILED");
    await closeState().catch(() => undefined);
    throw result;
  }
}

export async function validateCodexWorkspace(workingDirectory) {
  try {
    const directory = await realpath(workingDirectory);
    const [workspace, home, git] = await Promise.all([
      stat(directory),
      realpath(homedir()),
      stat(join(directory, ".git")),
    ]);
    if (
      !workspace.isDirectory() ||
      directory === parse(directory).root ||
      directory === home ||
      (!git.isDirectory() && !git.isFile())
    ) {
      throw failure("WORKSPACE_NOT_ALLOWED");
    }
    return directory;
  } catch (error) {
    if (error instanceof CodexControlFailure) throw error;
    throw failure("WORKSPACE_NOT_ALLOWED");
  }
}

function validateThread(value, cwd, threadSource) {
  const candidate = record(value);
  if (
    !candidate ||
    candidate.cwd !== cwd ||
    candidate.threadSource !== threadSource ||
    candidate.ephemeral !== false
  ) {
    throw failure("CODEX_PROTOCOL_MALFORMED");
  }
  return opaque(candidate.id);
}

export async function prepareCodexWork({
  intentId,
  workingDirectory,
  command = "codex",
  timeoutMs = 5_000,
}) {
  opaque(intentId);
  const cwd = await validateCodexWorkspace(workingDirectory);
  const threadSource = `patchfleet:${intentId}`;
  if (!SAFE_THREAD_SOURCE.test(threadSource)) throw failure("CODEX_PROTOCOL_MALFORMED");
  const active = await client(command, timeoutMs);
  return request(active, "thread/start", {
    cwd,
    approvalPolicy: "never",
    sandbox: "workspace-write",
    ephemeral: false,
    threadSource,
    experimentalRawEvents: false,
  }, true, (result) => {
    if (!record(result)) throw failure("CODEX_PROTOCOL_MALFORMED");
    const providerSessionId = validateThread(result.thread, cwd, threadSource);
    state.threads.add(providerSessionId);
    return { providerSessionId };
  });
}

export async function startCodexWork({
  intentId,
  instruction,
  providerSessionId,
  workingDirectory,
  command = "codex",
  timeoutMs = 5_000,
}) {
  opaque(intentId);
  await validateCodexWorkspace(workingDirectory);
  const threadSource = `patchfleet:${intentId}`;
  if (!SAFE_THREAD_SOURCE.test(threadSource)) throw failure("CODEX_PROTOCOL_MALFORMED");
  opaque(providerSessionId);
  if (!state.threads.has(providerSessionId)) {
    throw failure("CODEX_SESSION_LOST", { outcomeUnknown: true });
  }
  const active = await client(command, timeoutMs);

  return request(active, "turn/start", {
    threadId: providerSessionId,
    clientUserMessageId: intentId,
    input: [{ type: "text", text: instruction, text_elements: [] }],
  }, true, (result) => {
    if (!record(result)) throw failure("CODEX_PROTOCOL_MALFORMED");
    return { providerSessionId, ...turn(result.turn) };
  });
}

export async function cancelCodexRun({
  runId,
  providerSessionId,
  providerTurnId,
  workingDirectory,
  command = "codex",
  timeoutMs = 5_000,
}) {
  await validateCodexWorkspace(workingDirectory);
  opaque(runId);
  opaque(providerSessionId);
  if (!state.threads.has(providerSessionId)) {
    throw failure("CODEX_SESSION_LOST", { outcomeUnknown: true });
  }
  opaque(providerTurnId);
  const active = await client(command, timeoutMs);

  await request(active, "turn/interrupt", {
    threadId: providerSessionId,
    turnId: providerTurnId,
  }, true, (result) => {
    if (!record(result)) throw failure("CODEX_PROTOCOL_MALFORMED");
  });
}
