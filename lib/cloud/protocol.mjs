import { validateCommandIntent, validateCommandReceipt } from "../domain/work.mjs";

export const PROTOCOL_VERSION = 2;

const OPAQUE = /^[A-Za-z0-9._:-]{1,256}$/;
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;
const PROVIDERS = new Set(["codex", "claude", "gemini"]);
const PROVIDER_STATES = new Set(["available", "degraded", "unavailable"]);
const WORK_STATES = new Set([
  "queued", "launching", "running", "cancelling", "blocked",
  "completed", "failed", "interrupted",
]);
const RUN_STATES = new Set(["running", "cancelling", "completed", "failed", "interrupted"]);
const TERMINAL_STATES = new Set(["completed", "failed", "interrupted"]);

function record(value, name) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${name} must be an object`);
  }
  return value;
}

function exact(value, fields, name) {
  const keys = Object.keys(record(value, name));
  if (keys.length !== fields.length || !keys.every((key) => fields.includes(key))) {
    throw new TypeError(`${name} fields must match`);
  }
}

function opaque(value, name) {
  if (typeof value !== "string" || !OPAQUE.test(value)) throw new TypeError(`${name} is invalid`);
  return value;
}

function version(value) {
  if (typeof value !== "string" || value.length > 256 || !VERSION.test(value)) {
    throw new TypeError("provider version is invalid");
  }
  return value;
}

function iso(value, name) {
  if (typeof value !== "string" || new Date(value).toISOString() !== value) {
    throw new TypeError(`${name} is invalid`);
  }
  return value;
}

function integer(value, name, minimum = 0) {
  if (!Number.isSafeInteger(value) || value < minimum) throw new TypeError(`${name} is invalid`);
  return value;
}

function boolean(value, name) {
  if (typeof value !== "boolean") throw new TypeError(`${name} is invalid`);
  return value;
}

export function validatePairingResponse(value) {
  const input = record(value, "pairing response");
  exact(input, ["schemaVersion", "hostId", "workspaceId", "credential", "protocolVersion"], "pairing response");
  if (input.schemaVersion !== 1 || ![1, 2].includes(input.protocolVersion)) {
    throw new TypeError("unsupported pairing response");
  }
  return {
    schemaVersion: 1,
    hostId: opaque(input.hostId, "hostId"),
    workspaceId: opaque(input.workspaceId, "workspaceId"),
    credential: opaque(input.credential, "credential"),
    protocolVersion: input.protocolVersion,
  };
}

export function validateAcceptedResponse(value) {
  exact(value, ["schemaVersion", "accepted"], "accepted response");
  if (value.schemaVersion !== 1 || value.accepted !== true) throw new TypeError("message was not accepted");
  return { schemaVersion: 1, accepted: true };
}

export function validateIntentPage(value, hostId, after, now) {
  const input = record(value, "intent page");
  exact(input, ["schemaVersion", "hostId", "cursor", "intents"], "intent page");
  if (input.schemaVersion !== 1 || opaque(input.hostId, "hostId") !== hostId) {
    throw new TypeError("intent host mismatch");
  }
  if (!Array.isArray(input.intents) || input.intents.length > 20) throw new TypeError("invalid intents");

  const hostNow = Date.parse(iso(now, "host now"));
  let next = integer(after, "after");
  const intents = input.intents.map((candidate) => {
    exact(candidate, ["sequence", "intent"], "sequenced intent");
    const sequence = integer(candidate.sequence, "intent sequence", 1);
    if (sequence <= next) throw new TypeError("intent sequence replay or reordering");
    next = sequence;
    const intent = validateCommandIntent(candidate.intent);
    if (intent.type !== "cancel_run" || intent.actorId !== "cloud-owner") {
      throw new TypeError("unsupported remote intent");
    }
    const createdAt = Date.parse(intent.createdAt);
    if (Date.parse(intent.expiresAt) - createdAt > 5 * 60_000 || createdAt - hostNow > 60_000) {
      throw new TypeError("invalid remote intent time window");
    }
    return { sequence, intent };
  });
  if (integer(input.cursor, "cursor") !== next) throw new TypeError("intent cursor mismatch");
  return { schemaVersion: 1, hostId, cursor: next, intents };
}

export function buildCloudProjection(observation, work, revision) {
  const workItems = (work?.items ?? []).filter((item) => !TERMINAL_STATES.has(item.status));
  const runs = (work?.runs ?? []).filter((run) => !TERMINAL_STATES.has(run.status));
  const payload = {
    revision: integer(revision, "projection revision"),
    providers: (observation?.observations ?? []).map((candidate) => {
      const provider = record(candidate.provider, "provider");
      if (!PROVIDERS.has(provider.id) || !PROVIDER_STATES.has(provider.state)) {
        throw new TypeError("invalid provider projection");
      }
      const capabilities = record(provider.capabilities, "capabilities");
      return {
        providerId: provider.id,
        state: provider.state,
        version: provider.version === null ? null : version(provider.version),
        capabilities: {
          recentObservation: boolean(capabilities.recentObservation, "recent observation capability"),
          explicitLiveStatus: boolean(capabilities.explicitLiveStatus, "live status capability"),
        },
        observedAt: iso(candidate.observedAt, "provider observedAt"),
      };
    }),
    workItems: workItems.slice(0, 32).map((item, queuePosition) => {
      if (item.providerId !== "codex" || !WORK_STATES.has(item.status)) {
        throw new TypeError("invalid work projection");
      }
      return {
        workItemId: opaque(item.workItemId, "workItemId"),
        providerId: "codex",
        status: item.status,
        revision: integer(item.revision, "work revision", 1),
        queuePosition,
        createdAt: iso(item.createdAt, "work createdAt"),
      };
    }),
    runs: runs.slice(0, 32).map((run) => {
      if (run.providerId !== "codex" || !RUN_STATES.has(run.status)) {
        throw new TypeError("invalid run projection");
      }
      return {
        runId: opaque(run.runId, "runId"),
        workItemId: opaque(run.workItemId, "workItemId"),
        providerId: "codex",
        status: run.status,
        revision: integer(run.revision, "run revision", 1),
        startedAt: iso(run.startedAt, "run startedAt"),
        terminalAt: run.terminalAt === null ? null : iso(run.terminalAt, "run terminalAt"),
      };
    }),
    workItemsTruncated: workItems.length > 32,
    runsTruncated: runs.length > 32,
  };
  return payload;
}

export function buildReceiptPayload(receipt) {
  const normalized = validateCommandReceipt(receipt);
  if (normalized.commandType !== "cancel_run") throw new TypeError("invalid remote receipt");
  return { receipts: [normalized] };
}
