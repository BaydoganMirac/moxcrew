import { randomUUID } from "node:crypto";
import { mkdir, open, readFile, rename } from "node:fs/promises";
import { join } from "node:path";
import {
  normalizeObservation,
  observationProvider,
  safeObservationError,
  validateProjection,
} from "../domain/observation.mjs";
import { validateProviderLifecycleSignal } from "../domain/provider-lifecycle-signal.mjs";
import {
  projectWorkEvents,
  validateWorkEvent,
  validateWorkProjection,
} from "../domain/work.mjs";
import {
  projectWorkspaceEvents,
  validateWorkspaceEvent,
  validateWorkspaceProjection,
} from "../domain/workspace.mjs";
import { resolvePatchfleetDataDirectory } from "./gemini-inbox.mjs";

const EVENT_FILE = "events.jsonl";
const PROJECTION_FILE = "observation.json";
const WORK_PROJECTION_FILE = "work-items.json";
const WORKSPACE_PROJECTION_FILE = "workspaces.json";
const EVENT_TYPES = new Set([
  "observation.failed",
  "provider.observed",
  "session.observed",
  "session.terminal",
]);
const WORK_EVENT_TYPES = new Set([
  "command.receipted",
  "command.requested",
  "run.interrupted",
  "run.launching",
  "run.prepared",
  "run.session_lost",
  "run.start_unknown",
  "run.started",
  "turn.requested",
  "work.enqueued",
  "work.removed",
]);
const WORKSPACE_EVENT_TYPES = new Set([
  "workspace.command.receipted",
  "workspace.command.requested",
  "workspace.registered",
  "workspace.removed",
]);
const TERMINAL_STATES = new Set(["completed", "failed", "interrupted"]);
const PROVIDER_ORDER = ["codex", "claude", "gemini"];
const OPAQUE_ID = /^[A-Za-z0-9._:-]{1,256}$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const writerKey = Symbol.for("patchfleet.observation-writer");
const writer = (globalThis[writerKey] ??= { tail: Promise.resolve() });

export class StorageCorruptionError extends Error {
  constructor() {
    super("Local observation storage is corrupt");
    this.code = "LOCAL_STORAGE_CORRUPT";
  }
}

function dataDirectory(override) {
  return resolvePatchfleetDataDirectory(override);
}

function record(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError();
  return value;
}

function exact(value, keys) {
  const actual = Object.keys(record(value));
  if (actual.some((key) => !keys.includes(key))) throw new TypeError();
}

function iso(value) {
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError();
  return value;
}

function validateError(providerId, value) {
  exact(value, ["code", "message"]);
  const safe = safeObservationError(providerId, value.code);
  if (value.message !== safe.message) throw new TypeError();
  return safe;
}

function sessionKey(providerId, providerSessionId) {
  return `${providerId}\0${providerSessionId}`;
}

function recentSessions(sessions) {
  return [...sessions]
    .sort((left, right) =>
      right.lastObservedAt.localeCompare(left.lastObservedAt) ||
      left.providerSessionId.localeCompare(right.providerSessionId))
    .slice(0, 20);
}

function validateEvent(value) {
  exact(value, ["id", "schemaVersion", "type", "recordedAt", "payload"]);
  if (
    !UUID.test(value.id) ||
    value.schemaVersion !== 1 ||
    (!EVENT_TYPES.has(value.type) && !WORK_EVENT_TYPES.has(value.type) && !WORKSPACE_EVENT_TYPES.has(value.type))
  ) {
    throw new TypeError();
  }
  iso(value.recordedAt);
  if (WORK_EVENT_TYPES.has(value.type)) return validateWorkEvent(value);
  if (WORKSPACE_EVENT_TYPES.has(value.type)) return validateWorkspaceEvent(value);
  const payload = record(value.payload);
  const identity = observationProvider(payload.providerId);

  if (value.type === "provider.observed") {
    exact(payload, ["providerId", "state", "version", "capabilities", "error", "observedAt", "sessionIds"]);
    if (!Array.isArray(payload.sessionIds)) throw new TypeError();
    exact(payload.capabilities, ["recentObservation", "explicitLiveStatus"]);
    if (payload.error) validateError(payload.providerId, payload.error);
    if (new Set(payload.sessionIds).size !== payload.sessionIds.length) throw new TypeError();
    for (const id of payload.sessionIds) if (!OPAQUE_ID.test(id)) throw new TypeError();
    normalizeObservation({
      provider: { ...identity, ...payload },
      observedAt: payload.observedAt,
      sessions: [],
    });
  } else if (value.type === "session.observed") {
    exact(payload, ["providerId", "providerSessionId", "status", "createdAt", "lastObservedAt", "terminalAt"]);
    normalizeObservation({
      provider: {
        ...identity,
        state: "available",
        version: "0.0.0",
        capabilities: { recentObservation: true, explicitLiveStatus: true },
      },
      observedAt: payload.lastObservedAt,
      sessions: [payload],
    });
  } else if (value.type === "session.terminal") {
    exact(payload, ["providerId", "providerSessionId", "status", "lastObservedAt", "terminalAt"]);
    if (
      !OPAQUE_ID.test(payload.providerSessionId) ||
      !TERMINAL_STATES.has(payload.status)
    ) {
      throw new TypeError();
    }
    iso(payload.lastObservedAt);
    if (payload.terminalAt !== undefined) iso(payload.terminalAt);
  } else {
    exact(payload, ["providerId", "observedAt", "error"]);
    iso(payload.observedAt);
    validateError(payload.providerId, payload.error);
  }
  return value;
}

function event(type, recordedAt, payload) {
  return validateEvent({ id: randomUUID(), schemaVersion: 1, type, recordedAt, payload });
}

async function readLog(directory) {
  let bytes;
  try {
    bytes = await readFile(join(directory, EVENT_FILE));
  } catch (error) {
    if (error.code === "ENOENT") return { events: [], validBytes: 0, incompleteTail: false };
    throw error;
  }

  const incompleteTail = bytes.length > 0 && bytes.at(-1) !== 10;
  const lastNewline = bytes.lastIndexOf(10);
  const validBytes = incompleteTail ? lastNewline + 1 : bytes.length;
  const text = bytes.subarray(0, validBytes).toString("utf8");
  const lines = text ? text.slice(0, -1).split("\n") : [];
  const events = [];
  const ids = new Set();
  try {
    for (const line of lines) {
      if (!line) throw new TypeError();
      const parsed = validateEvent(JSON.parse(line));
      if (ids.has(parsed.id)) throw new TypeError();
      ids.add(parsed.id);
      events.push(parsed);
    }
  } catch {
    throw new StorageCorruptionError();
  }
  return { events, validBytes, incompleteTail };
}

export async function replayEvents({ dataDir } = {}) {
  return (await readLog(dataDirectory(dataDir))).events;
}

export function projectEvents(events) {
  const sessions = new Map();
  const providers = new Map();
  const currentIds = new Map();

  try {
    for (const candidate of events) {
      const item = validateEvent(candidate);
      const payload = item.payload;
      if (item.type === "session.observed") {
        sessions.set(sessionKey(payload.providerId, payload.providerSessionId), {
          providerSessionId: payload.providerSessionId,
          status: payload.status,
          createdAt: payload.createdAt,
          lastObservedAt: payload.lastObservedAt,
          ...(payload.terminalAt ? { terminalAt: payload.terminalAt } : {}),
        });
      } else if (item.type === "session.terminal") {
        const key = sessionKey(payload.providerId, payload.providerSessionId);
        const session = sessions.get(key);
        if (!session) throw new TypeError();
        sessions.set(key, {
          ...session,
          status: payload.status,
          lastObservedAt: payload.lastObservedAt,
          ...(payload.terminalAt ? { terminalAt: payload.terminalAt } : {}),
        });
      } else if (item.type === "provider.observed") {
        if (payload.sessionIds.some((id) => !sessions.has(sessionKey(payload.providerId, id)))) {
          throw new TypeError();
        }
        currentIds.set(payload.providerId, payload.sessionIds);
        providers.set(payload.providerId, {
          ...observationProvider(payload.providerId),
          state: payload.state,
          version: payload.version,
          capabilities: payload.capabilities,
          ...(payload.error ? { error: payload.error } : {}),
          observedAt: payload.observedAt,
        });
      }
    }
    if (!providers.size) return null;
    return {
      schemaVersion: 2,
      observations: PROVIDER_ORDER.filter((providerId) => providers.has(providerId)).map(
        (providerId) => {
          const provider = providers.get(providerId);
          return normalizeObservation({
            provider,
            observedAt: provider.observedAt,
            sessions: currentIds
              .get(providerId)
              .map((id) => sessions.get(sessionKey(providerId, id))),
          });
        },
      ),
    };
  } catch (error) {
    if (error instanceof StorageCorruptionError) throw error;
    throw new StorageCorruptionError();
  }
}

async function writeProjection(directory, filename, projection) {
  const target = join(directory, filename);
  const temporary = `${target}.${randomUUID()}.tmp`;
  const handle = await open(temporary, "wx", 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(projection)}\n`, "utf8");
    await handle.sync();
  } finally {
    await handle.close();
  }
  await rename(temporary, target);
}

async function appendInternal(directory, additions, state) {
  const existingIds = new Set(state.events.map((item) => item.id));
  for (const item of additions) {
    validateEvent(item);
    if (existingIds.has(item.id)) throw new StorageCorruptionError();
    existingIds.add(item.id);
  }

  const combined = [...state.events, ...additions];
  const observationProjection = projectEvents(combined);
  let workProjection;
  let workspaceProjection;
  try {
    workProjection = projectWorkEvents(combined);
    workspaceProjection = projectWorkspaceEvents(combined);
  } catch {
    throw new StorageCorruptionError();
  }

  await mkdir(directory, { recursive: true, mode: 0o700 });
  const logPath = join(directory, EVENT_FILE);
  if (state.incompleteTail) {
    const recovery = await open(logPath, "r+");
    try {
      await recovery.truncate(state.validBytes);
      await recovery.sync();
    } finally {
      await recovery.close();
    }
  }

  if (additions.length) {
    const log = await open(logPath, "a", 0o600);
    try {
      await log.writeFile(`${additions.map((item) => JSON.stringify(item)).join("\n")}\n`, "utf8");
      await log.sync();
    } finally {
      await log.close();
    }
  }
  if (observationProjection) await writeProjection(directory, PROJECTION_FILE, observationProjection);
  if (workProjection) await writeProjection(directory, WORK_PROJECTION_FILE, workProjection);
  if (workspaceProjection) await writeProjection(directory, WORKSPACE_PROJECTION_FILE, workspaceProjection);
  return { observationProjection, workProjection, workspaceProjection };
}

function serialized(operation) {
  const result = writer.tail.then(operation, operation);
  writer.tail = result.then(
    () => undefined,
    () => undefined,
  );
  return result;
}

export function appendEvents(additions, { dataDir } = {}) {
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    return (await appendInternal(directory, additions, await readLog(directory))).observationProjection;
  });
}

export function commitEventTransaction(build, { dataDir } = {}) {
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    const state = await readLog(directory);
    try {
      projectWorkEvents(state.events);
      projectWorkspaceEvents(state.events);
    } catch {
      throw new StorageCorruptionError();
    }
    const transaction = build(Object.freeze([...state.events]));
    if (!transaction || !Array.isArray(transaction.additions) || typeof transaction.result !== "function") {
      throw new TypeError("invalid event transaction");
    }
    const projections = await appendInternal(directory, transaction.additions, state);
    return transaction.result(projections);
  });
}

export function persistObservation(value, { dataDir, preserveSessions = false } = {}) {
  const requested = normalizeObservation(value);
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    const state = await readLog(directory);
    let projectedSessions = requested.sessions;
    if (preserveSessions) {
      const current = projectEvents(state.events)?.observations.find(
        (item) => item.provider.id === requested.provider.id,
      );
      const sessions = new Map(
        (current?.sessions ?? []).map((session) => [session.providerSessionId, session]),
      );
      for (const session of requested.sessions) sessions.set(session.providerSessionId, session);
      projectedSessions = recentSessions(sessions.values());
    }
    const latestStatus = new Map();
    for (const item of state.events) {
      if (item.type === "session.observed" || item.type === "session.terminal") {
        latestStatus.set(
          sessionKey(item.payload.providerId, item.payload.providerSessionId),
          item.payload.status,
        );
      }
    }
    const additions = [];
    const providerId = requested.provider.id;

    for (const session of requested.sessions) {
      additions.push(event("session.observed", requested.observedAt, {
        providerId,
        ...session,
      }));
      const key = sessionKey(providerId, session.providerSessionId);
      if (
        TERMINAL_STATES.has(session.status) &&
        latestStatus.get(key) !== session.status
      ) {
        additions.push(event("session.terminal", requested.observedAt, {
          providerId,
          providerSessionId: session.providerSessionId,
          status: session.status,
          lastObservedAt: session.lastObservedAt,
          ...(session.terminalAt ? { terminalAt: session.terminalAt } : {}),
        }));
      }
      latestStatus.set(key, session.status);
    }

    if (requested.provider.error) {
      additions.push(event("observation.failed", requested.observedAt, {
        providerId,
        observedAt: requested.observedAt,
        error: requested.provider.error,
      }));
    }
    additions.push(event("provider.observed", requested.observedAt, {
      providerId,
      state: requested.provider.state,
      version: requested.provider.version,
      capabilities: requested.provider.capabilities,
      ...(requested.provider.error ? { error: requested.provider.error } : {}),
      observedAt: requested.observedAt,
      sessionIds: projectedSessions.map((session) => session.providerSessionId),
    }));

    return (await appendInternal(directory, additions, state)).observationProjection;
  });
}

export function persistGeminiLifecycleSignal(value, { dataDir } = {}) {
  const signal = validateProviderLifecycleSignal(value);
  if (signal.providerId !== "gemini") throw new TypeError("invalid Gemini lifecycle signal");

  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    const state = await readLog(directory);
    // ponytail: scan the V0 JSON log; add an index only when measured size hurts refresh.
    const duplicate = state.events.some((item) =>
      item.type === "session.observed" &&
      item.payload.providerId === "gemini" &&
      item.payload.providerSessionId === signal.providerSessionId &&
      item.payload.status === signal.status &&
      item.payload.createdAt === null &&
      item.payload.lastObservedAt === signal.observedAt &&
      item.payload.terminalAt === undefined);
    if (duplicate) return projectEvents(state.events);

    const current = projectEvents(state.events)?.observations.find(
      (item) => item.provider.id === "gemini",
    );
    if (!current || current.provider.state !== "available") {
      throw new TypeError("Gemini extension is not active");
    }

    const next = {
      providerSessionId: signal.providerSessionId,
      status: signal.status,
      createdAt: null,
      lastObservedAt: signal.observedAt,
    };
    const sessions = new Map(
      current.sessions.map((session) => [session.providerSessionId, session]),
    );
    sessions.set(signal.providerSessionId, next);
    const retained = recentSessions(sessions.values());
    const additions = [
      event("session.observed", signal.observedAt, { providerId: "gemini", ...next }),
      event("provider.observed", current.observedAt, {
        providerId: "gemini",
        state: current.provider.state,
        version: current.provider.version,
        capabilities: current.provider.capabilities,
        observedAt: current.observedAt,
        sessionIds: retained.map((session) => session.providerSessionId),
      }),
    ];
    return (await appendInternal(directory, additions, state)).observationProjection;
  });
}

export function rebuildProjection({ dataDir } = {}) {
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    const events = (await readLog(directory)).events;
    const projection = projectEvents(events);
    let workProjection;
    let workspaceProjection;
    try {
      workProjection = projectWorkEvents(events);
      workspaceProjection = projectWorkspaceEvents(events);
    } catch {
      throw new StorageCorruptionError();
    }
    if (projection || workProjection || workspaceProjection) {
      await mkdir(directory, { recursive: true, mode: 0o700 });
      if (projection) await writeProjection(directory, PROJECTION_FILE, projection);
      if (workProjection) await writeProjection(directory, WORK_PROJECTION_FILE, workProjection);
      if (workspaceProjection) await writeProjection(directory, WORKSPACE_PROJECTION_FILE, workspaceProjection);
    }
    return projection;
  });
}

export function rebuildWorkProjection({ dataDir } = {}) {
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    let projection;
    try {
      projection = projectWorkEvents((await readLog(directory)).events);
    } catch {
      throw new StorageCorruptionError();
    }
    if (projection) {
      await mkdir(directory, { recursive: true, mode: 0o700 });
      await writeProjection(directory, WORK_PROJECTION_FILE, projection);
    }
    return projection;
  });
}

export async function readProjection({ dataDir } = {}) {
  const directory = dataDirectory(dataDir);
  try {
    await readLog(directory);
    const content = await readFile(join(directory, PROJECTION_FILE), "utf8");
    return validateProjection(JSON.parse(content));
  } catch (error) {
    if (error.code === "ENOENT") return null;
    if (error instanceof StorageCorruptionError) throw error;
    throw new StorageCorruptionError();
  }
}

export function readWorkProjection({ dataDir } = {}) {
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    let canonical;
    try {
      canonical = projectWorkEvents((await readLog(directory)).events);
    } catch (error) {
      if (error instanceof StorageCorruptionError) throw error;
      throw new StorageCorruptionError();
    }
    if (!canonical) return null;

    let stored;
    try {
      stored = validateWorkProjection(JSON.parse(
        await readFile(join(directory, WORK_PROJECTION_FILE), "utf8"),
      ));
    } catch (error) {
      if (error.code && error.code !== "ENOENT") throw error;
    }
    if (JSON.stringify(stored) !== JSON.stringify(canonical)) {
      await mkdir(directory, { recursive: true, mode: 0o700 });
      await writeProjection(directory, WORK_PROJECTION_FILE, canonical);
    }
    return canonical;
  });
}

export function readWorkspaceProjection({ dataDir } = {}) {
  return serialized(async () => {
    const directory = dataDirectory(dataDir);
    let canonical;
    try {
      canonical = projectWorkspaceEvents((await readLog(directory)).events);
    } catch (error) {
      if (error instanceof StorageCorruptionError) throw error;
      throw new StorageCorruptionError();
    }
    if (!canonical) return null;

    let stored;
    try {
      stored = validateWorkspaceProjection(JSON.parse(
        await readFile(join(directory, WORKSPACE_PROJECTION_FILE), "utf8"),
      ));
    } catch (error) {
      if (error.code && error.code !== "ENOENT") throw error;
    }
    if (JSON.stringify(stored) !== JSON.stringify(canonical)) {
      await mkdir(directory, { recursive: true, mode: 0o700 });
      await writeProjection(directory, WORKSPACE_PROJECTION_FILE, canonical);
    }
    return canonical;
  });
}
