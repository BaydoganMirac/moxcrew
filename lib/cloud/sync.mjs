import { createHash, randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { platform } from "node:os";
import { join } from "node:path";
import { projectWorkEvents } from "../domain/work.mjs";
import { supportsCodexControl } from "../providers/codex.mjs";
import { projectEvents, replayEvents } from "../runtime/observation-store.mjs";
import { applyWorkControlCommand } from "../runtime/work-queue.mjs";
import { readCloudState, writeCloudState, normalizeCloudUrl } from "./connection.mjs";
import {
  PROTOCOL_VERSION,
  buildCloudProjection,
  buildReceiptPayload,
  validateAcceptedResponse,
  validateIntentPage,
  validatePairingResponse,
} from "./protocol.mjs";

const MAX_JSON_BYTES = 65_536;
const cloudKey = Symbol.for("patchfleet.cloud-writer");
const cloudWriter = (globalThis[cloudKey] ??= { tail: Promise.resolve() });
const triggerKey = Symbol.for("patchfleet.cloud-sync-trigger");
const trigger = (globalThis[triggerKey] ??= { pending: null });
let versionPromise;

function serialized(operation) {
  const result = cloudWriter.tail.then(operation, operation);
  cloudWriter.tail = result.catch(() => undefined);
  return result;
}

export function triggerCloudSync(operation = () => syncCloud()) {
  if (trigger.pending) return { accepted: false };
  const pending = Promise.resolve().then(operation);
  trigger.pending = pending;
  void pending.finally(() => {
    if (trigger.pending === pending) trigger.pending = null;
  }).catch(() => undefined);
  return { accepted: true, result: pending };
}

function appVersion() {
  versionPromise ??= readFile(join(process.cwd(), "package.json"), "utf8")
    .then((text) => JSON.parse(text).version);
  return versionPromise;
}

function osFamily() {
  return platform() === "win32" ? "windows" : platform() === "darwin" ? "darwin" : "linux";
}

function exactJsonHeaders(credential) {
  return {
    "content-type": "application/json",
    ...(credential ? { authorization: `Bearer ${credential}` } : {}),
  };
}

async function readJson(response) {
  if (!/^application\/json(?:;\s*charset=utf-8)?$/i.test(response.headers.get("content-type") ?? "")) {
    throw Object.assign(new Error("Cloud returned an invalid content type"), { code: "CLOUD_PROTOCOL_INVALID" });
  }
  const reader = response.body?.getReader();
  if (!reader) throw Object.assign(new Error("Cloud returned no response body"), { code: "CLOUD_PROTOCOL_INVALID" });
  const chunks = [];
  let bytes = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    bytes += value.byteLength;
    if (bytes > MAX_JSON_BYTES) {
      await reader.cancel();
      throw Object.assign(new Error("Cloud response is too large"), { code: "CLOUD_PROTOCOL_INVALID" });
    }
    chunks.push(value);
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    throw Object.assign(new Error("Cloud returned invalid JSON"), { code: "CLOUD_PROTOCOL_INVALID" });
  }
}

async function requestJson(fetchImpl, url, { method, credential, body }) {
  const encoded = body === undefined ? undefined : JSON.stringify(body);
  if (encoded !== undefined && Buffer.byteLength(encoded) > MAX_JSON_BYTES) {
    throw Object.assign(new Error("Cloud request is too large"), { code: "CLOUD_PROTOCOL_INVALID" });
  }
  let response;
  try {
    response = await fetchImpl(url, {
      method,
      headers: exactJsonHeaders(credential),
      ...(encoded === undefined ? {} : { body: encoded }),
      redirect: "error",
      signal: AbortSignal.timeout(5_000),
    });
  } catch {
    throw Object.assign(new Error("Patchfleet Cloud is unavailable"), { code: "CLOUD_UNAVAILABLE" });
  }
  if (!response.ok) {
    let reason = null;
    if (response.status === 409) {
      try {
        const candidate = await readJson(response);
        if (candidate && typeof candidate === "object" && typeof candidate.error === "string") reason = candidate.error;
      } catch {
        // The stable status code remains sufficient when Cloud does not return a valid error body.
      }
    }
    const code = response.status === 401 || response.status === 403
      ? "CLOUD_AUTH_REJECTED"
      : response.status === 409
        ? "CLOUD_CONFLICT"
        : "CLOUD_REQUEST_REJECTED";
    throw Object.assign(new Error("Patchfleet Cloud rejected the request"), { code, reason });
  }
  return readJson(response);
}

function envelope(connection, messageId, occurredAt, payload) {
  return { schemaVersion: 1, messageId, hostId: connection.hostId, occurredAt, payload };
}

function stableMessageId(kind, value) {
  return `${kind}:${createHash("sha256").update(value).digest("hex")}`;
}

async function localSnapshot(dataDir) {
  const events = await replayEvents({ dataDir });
  return {
    events,
    observation: projectEvents(events),
    work: projectWorkEvents(events),
  };
}

export function pairCloud({ cloudUrl, pairingCode, displayName }, {
  dataDir,
  fetchImpl = fetch,
  now = () => new Date().toISOString(),
} = {}) {
  return serialized(async () => {
    const state = await readCloudState({ dataDir });
    if (state.connection) throw new TypeError("disconnect before pairing again");
    const origin = normalizeCloudUrl(cloudUrl);
    if (typeof pairingCode !== "string" || pairingCode.length < 1 || pairingCode.length > 256 || pairingCode.trim() !== pairingCode) {
      throw new TypeError("invalid pairing code");
    }
    if (typeof displayName !== "string" || displayName.length < 1 || displayName.length > 80 || displayName.trim() !== displayName) {
      throw new TypeError("invalid display name");
    }
    await writeCloudState(state, { dataDir });
    const response = validatePairingResponse(await requestJson(fetchImpl, `${origin}/api/v1/pairings/consume`, {
      method: "POST",
      body: {
        schemaVersion: 1,
        messageId: `pair:${randomUUID()}`,
        occurredAt: now(),
        pairingCode,
        installationId: state.installationId,
        displayName,
        appVersion: await appVersion(),
        protocolVersion: PROTOCOL_VERSION,
        osFamily: osFamily(),
      },
    }));
    return writeCloudState({
      ...state,
      connection: {
        cloudUrl: origin,
        hostId: response.hostId,
        workspaceId: response.workspaceId,
        credential: response.credential,
        protocolVersion: 1,
        cursor: 0,
        projectionRevisionOffset: 0,
        lastAttemptAt: null,
        lastSuccessAt: null,
        lastErrorCode: null,
      },
    }, { dataDir });
  });
}

export function disconnectCloud({ dataDir } = {}) {
  return serialized(async () => {
    const state = await readCloudState({ dataDir });
    return writeCloudState({ ...state, connection: null }, { dataDir });
  });
}

export function syncCloud({
  dataDir,
  fetchImpl = fetch,
  now = () => new Date().toISOString(),
  loadSnapshot = () => localSnapshot(dataDir),
  applyIntent = (intent, options) => applyWorkControlCommand(intent, options),
} = {}) {
  return serialized(async () => {
    let state = await readCloudState({ dataDir });
    if (!state.connection) return { kind: "unpaired" };
    const attemptedAt = now();
    let connection = state.connection;
    try {
      const snapshot = await loadSnapshot();
      const localSequence = snapshot.events.length;
      let revision = localSequence + connection.projectionRevisionOffset;
      const occurredAt = snapshot.events.at(-1)?.recordedAt ?? "1970-01-01T00:00:00.000Z";
      let payload = buildCloudProjection(snapshot.observation, snapshot.work, revision);
      const version = await appVersion();

      validateAcceptedResponse(await requestJson(fetchImpl, `${connection.cloudUrl}/api/v1/hosts/${encodeURIComponent(connection.hostId)}/heartbeat`, {
        method: "POST",
        credential: connection.credential,
        body: envelope(connection, `heartbeat:${randomUUID()}`, attemptedAt, {
          appVersion: version,
          protocolVersion: 1,
          lastLocalSequence: localSequence,
          health: "ok",
        }),
      }));
      const sendProjection = () => requestJson(fetchImpl, `${connection.cloudUrl}/api/v1/hosts/${encodeURIComponent(connection.hostId)}/projection`, {
        method: "PUT",
        credential: connection.credential,
        body: envelope(
          connection,
          stableMessageId("projection", `${connection.hostId}:${revision}`),
          occurredAt,
          payload,
        ),
      });
      try {
        validateAcceptedResponse(await sendProjection());
      } catch (error) {
        const equalRevisionConflict = error?.code === "CLOUD_CONFLICT" && [
          "conflicting message replay",
          "conflicting projection revision",
        ].includes(error?.reason);
        if (!equalRevisionConflict) throw error;
        revision += 1;
        payload = { ...payload, revision };
        validateAcceptedResponse(await sendProjection());
        connection = { ...connection, projectionRevisionOffset: connection.projectionRevisionOffset + 1 };
        state = await writeCloudState({ ...state, connection }, { dataDir });
      }

      const page = validateIntentPage(await requestJson(
        fetchImpl,
        `${connection.cloudUrl}/api/v1/hosts/${encodeURIComponent(connection.hostId)}/intents?after=${connection.cursor}`,
        { method: "GET", credential: connection.credential },
      ), connection.hostId, connection.cursor, attemptedAt);

      let cursor = connection.cursor;
      const providerAvailable = snapshot.observation?.observations.some(supportsCodexControl) ?? false;
      for (const { sequence, intent } of page.intents) {
        const receipt = await applyIntent(intent, { dataDir, providerAvailable });
        const receiptBody = envelope(
          connection,
          stableMessageId("receipt", receipt.intentId),
          receipt.completedAt,
          buildReceiptPayload(receipt),
        );
        validateAcceptedResponse(await requestJson(fetchImpl, `${connection.cloudUrl}/api/v1/hosts/${encodeURIComponent(connection.hostId)}/receipts`, {
          method: "POST",
          credential: connection.credential,
          body: receiptBody,
        }));
        cursor = sequence;
        state = await writeCloudState({
          ...state,
          connection: { ...connection, cursor, lastAttemptAt: attemptedAt, lastSuccessAt: now(), lastErrorCode: null },
        }, { dataDir });
      }
      state = await writeCloudState({
        ...state,
        connection: { ...connection, cursor, lastAttemptAt: attemptedAt, lastSuccessAt: now(), lastErrorCode: null },
      }, { dataDir });
      return { kind: "synced", cursor, revision };
    } catch (error) {
      const lastErrorCode = typeof error?.code === "string" && /^CLOUD_[A-Z0-9_]{1,57}$/.test(error.code)
        ? error.code
        : "CLOUD_SYNC_FAILED";
      await writeCloudState({
        ...state,
        connection: { ...state.connection, lastAttemptAt: attemptedAt, lastErrorCode },
      }, { dataDir });
      return { kind: "failed", code: lastErrorCode };
    }
  });
}
