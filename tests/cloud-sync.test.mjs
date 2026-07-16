import assert from "node:assert/strict";
import { mkdtemp, readFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { normalizeCloudUrl, readCloudState } from "../lib/cloud/connection.mjs";
import { buildCloudProjection, validateIntentPage } from "../lib/cloud/protocol.mjs";
import { pairCloud, syncCloud, triggerCloudSync } from "../lib/cloud/sync.mjs";
import { applyWorkCommand, applyWorkControlCommand } from "../lib/runtime/work-queue.mjs";

const FIRST = "2026-07-16T12:00:00.000Z";
const SECOND = "2026-07-16T12:01:00.000Z";
const THIRD = "2026-07-16T12:02:00.000Z";
const EXPIRES = "2026-07-16T13:00:00.000Z";
const REMOTE_EXPIRES = "2026-07-16T12:05:00.000Z";

function json(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function pairingResponse() {
  return {
    schemaVersion: 1,
    hostId: "host:one",
    workspaceId: "workspace:one",
    credential: "credential:one",
    protocolVersion: 1,
  };
}

function intent({ runId = "run:one", createdAt = FIRST, expiresAt = REMOTE_EXPIRES } = {}) {
  return {
    schemaVersion: 1,
    intentId: "intent:cancel:one",
    idempotencyKey: "intent:cancel:one",
    type: "cancel_run",
    actorId: "cloud-owner",
    createdAt,
    expiresAt,
    payload: { runId, expectedRunRevision: 1 },
  };
}

function snapshot(canary = "PRIVATE_CANARY") {
  return {
    events: [{ recordedAt: FIRST }, { recordedAt: SECOND }],
    observation: {
      schemaVersion: 2,
      observations: [{
        schemaVersion: 1,
        provider: {
          id: "codex",
          displayName: canary,
          state: "available",
          version: "0.144.1",
          capabilities: { recentObservation: true, explicitLiveStatus: true },
          native: { prompt: canary },
        },
        observedAt: FIRST,
        sessions: [{ transcript: canary }],
      }],
    },
    work: {
      schemaVersion: 1,
      revision: 3,
      items: [{
        workItemId: "work:one",
        title: canary,
        instruction: canary,
        workingDirectory: `/private/${canary}`,
        providerId: "codex",
        status: "running",
        createdAt: FIRST,
        revision: 2,
      }],
      runs: [{
        runId: "run:one",
        workItemId: "work:one",
        providerId: "codex",
        ownerEpoch: canary,
        providerSessionId: canary,
        providerTurnId: canary,
        status: "running",
        startedAt: SECOND,
        terminalAt: null,
        revision: 1,
      }],
      receipts: [{ providerOutput: canary }],
    },
  };
}

async function pairedDirectory() {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-cloud-"));
  let pairingBody;
  await pairCloud({
    cloudUrl: "https://cloud.example.com",
    pairingCode: "one-time-code",
    displayName: "My Mac",
  }, {
    dataDir,
    now: () => FIRST,
    fetchImpl: async (_url, options) => {
      pairingBody = JSON.parse(options.body);
      return json(pairingResponse());
    },
  });
  return { dataDir, pairingBody };
}

test("Cloud projection is a field allowlist and connection storage is owner-only", async () => {
  const canary = "PRIVATE_PROMPT_TRANSCRIPT_PATH_TOKEN_SOURCE_DIFF";
  const local = snapshot(canary);
  local.observation.observations[0].provider.version = "2.1.169-beta.1+build.7";
  const payload = buildCloudProjection(local.observation, local.work, local.events.length);
  assert.equal(JSON.stringify(payload).includes(canary), false);
  assert.equal(payload.providers[0].version, "2.1.169-beta.1+build.7");
  assert.deepEqual(Object.keys(payload), [
    "revision", "providers", "workItems", "runs", "workItemsTruncated", "runsTruncated",
  ]);
  assert.deepEqual(Object.keys(payload.workItems[0]), [
    "workItemId", "providerId", "status", "revision", "queuePosition", "createdAt",
  ]);
  assert.deepEqual(Object.keys(payload.runs[0]), [
    "runId", "workItemId", "providerId", "status", "revision", "startedAt", "terminalAt",
  ]);

  const { dataDir, pairingBody } = await pairedDirectory();
  assert.equal(pairingBody.pairingCode, "one-time-code");
  const mode = (await stat(join(dataDir, "cloud.json"))).mode & 0o777;
  assert.equal(mode, 0o600);
  const stored = await readFile(join(dataDir, "cloud.json"), "utf8");
  assert.equal(stored.includes("one-time-code"), false);
  assert.equal((await readCloudState({ dataDir })).connection.credential, "credential:one");

  assert.equal(normalizeCloudUrl("http://127.0.0.1:3000"), "http://127.0.0.1:3000");
  assert.equal(normalizeCloudUrl("http://[::1]:3000"), "http://[::1]:3000");
  for (const value of ["http://example.com", "https://user:pass@example.com", "https://example.com/path"]) {
    assert.throws(() => normalizeCloudUrl(value), TypeError);
  }
});

test("intent pages reject replay, reordering, unknown commands, and identity mismatch", () => {
  const valid = {
    schemaVersion: 1,
    hostId: "host:one",
    cursor: 7,
    intents: [{ sequence: 7, intent: intent() }],
  };
  assert.equal(validateIntentPage(valid, "host:one", 2, SECOND).cursor, 7, "workspace sequence gaps are valid");
  assert.equal(validateIntentPage({
    ...valid,
    intents: [{ sequence: 7, intent: intent({ expiresAt: SECOND }) }],
  }, "host:one", 2, THIRD).cursor, 7, "already-expired intents reach the receipt path");
  for (const candidate of [
    { ...valid, hostId: "host:other" },
    { ...valid, cursor: 2, intents: [{ sequence: 2, intent: intent() }] },
    { ...valid, intents: [{ sequence: 7, intent: { ...intent(), type: "execute_shell" } }] },
    { ...valid, intents: [{ sequence: 7, intent: { ...intent(), actorId: "local-owner" } }] },
    { ...valid, intents: [{ sequence: 7, intent: intent({ expiresAt: "2026-07-16T12:05:00.001Z" }) }] },
    { ...valid, intents: [{ sequence: 7, intent: intent({
      createdAt: "2026-07-16T12:02:00.001Z",
      expiresAt: "2026-07-16T12:07:00.001Z",
    }) }] },
  ]) {
    assert.throws(() => validateIntentPage(candidate, "host:one", 2, SECOND), TypeError);
  }
});

test("Cloud projection keeps at most 32 deterministic non-terminal work items and runs", () => {
  const local = snapshot();
  local.work.items = [
    ...Array.from({ length: 40 }, (_, index) => ({
      ...local.work.items[0],
      workItemId: `work:${String(index).padStart(2, "0")}`,
      status: "running",
    })),
    { ...local.work.items[0], workItemId: "work:terminal", status: "completed" },
  ];
  local.work.runs = [
    ...Array.from({ length: 40 }, (_, index) => ({
      ...local.work.runs[0],
      runId: `run:${String(index).padStart(2, "0")}`,
      workItemId: `work:${String(index).padStart(2, "0")}`,
    })),
    { ...local.work.runs[0], runId: "run:terminal", status: "failed", terminalAt: THIRD },
  ];
  const payload = buildCloudProjection(local.observation, local.work, 1);
  assert.equal(payload.workItems.length, 32);
  assert.equal(payload.runs.length, 32);
  assert.equal(payload.workItemsTruncated, true);
  assert.equal(payload.runsTruncated, true);
  assert.deepEqual(payload.workItems.map((item) => item.workItemId),
    Array.from({ length: 32 }, (_, index) => `work:${String(index).padStart(2, "0")}`));
  assert.deepEqual(payload.runs.map((run) => run.runId),
    Array.from({ length: 32 }, (_, index) => `run:${String(index).padStart(2, "0")}`));
});

test("pairing persists and reuses installation identity across a lost response", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-pair-retry-"));
  const installationIds = [];
  await assert.rejects(() => pairCloud({
    cloudUrl: "https://cloud.example.com",
    pairingCode: "first-code",
    displayName: "My Mac",
  }, {
    dataDir,
    now: () => FIRST,
    fetchImpl: async (_url, options) => {
      installationIds.push(JSON.parse(options.body).installationId);
      throw new Error("response lost");
    },
  }));
  const afterLoss = await readCloudState({ dataDir });
  assert.equal(afterLoss.connection, null);
  assert.equal((await stat(join(dataDir, "cloud.json"))).mode & 0o777, 0o600);

  await pairCloud({
    cloudUrl: "https://cloud.example.com",
    pairingCode: "replacement-code",
    displayName: "My Mac",
  }, {
    dataDir,
    now: () => SECOND,
    fetchImpl: async (_url, options) => {
      installationIds.push(JSON.parse(options.body).installationId);
      return json(pairingResponse());
    },
  });
  assert.deepEqual(installationIds, [afterLoss.installationId, afterLoss.installationId]);
});

test("the server sync trigger admits only one in-flight operation", async () => {
  let release;
  let calls = 0;
  const gate = new Promise((resolve) => { release = resolve; });
  const first = triggerCloudSync(async () => {
    calls += 1;
    await gate;
    return { kind: "synced" };
  });
  await Promise.resolve();
  const duplicate = triggerCloudSync(async () => { calls += 1; });
  assert.deepEqual([first.accepted, duplicate.accepted, calls], [true, false, 1]);
  release();
  assert.deepEqual(await first.result, { kind: "synced" });
  await Promise.resolve();
  const next = triggerCloudSync(async () => { calls += 1; return { kind: "next" }; });
  assert.equal(next.accepted, true);
  assert.deepEqual(await next.result, { kind: "next" });
  assert.equal(calls, 2);
});

test("a dropped receipt response retries the same receipt before advancing the cursor", async () => {
  const { dataDir } = await pairedDirectory();
  const receipts = [];
  let receiptAttempts = 0;
  let cancelCalls = 0;
  const remoteIntent = intent({ runId: "run:control-start" });
  await applyWorkCommand({
    schemaVersion: 1,
    intentId: "queue:one",
    idempotencyKey: "queue:one",
    type: "enqueue_work",
    actorId: "local-owner",
    createdAt: FIRST,
    expiresAt: EXPIRES,
    payload: { workItem: {
      schemaVersion: 1,
      workItemId: "work:one",
      title: "Local title",
      instruction: "Local instruction",
      providerId: "codex",
      workingDirectory: tmpdir(),
      status: "queued",
      createdAt: FIRST,
      revision: 1,
    } },
  }, { dataDir, now: () => FIRST });
  await applyWorkControlCommand({
    schemaVersion: 1,
    intentId: "control-start",
    idempotencyKey: "control-start",
    type: "start_work",
    actorId: "local-owner",
    createdAt: FIRST,
    expiresAt: EXPIRES,
    payload: { workItemId: "work:one", expectedItemRevision: 1 },
  }, {
    dataDir,
    now: () => SECOND,
    ownerEpoch: "cloud-test-owner",
    prepare: async () => ({ providerSessionId: "session:one" }),
    start: async () => ({
      providerSessionId: "session:one",
      providerTurnId: "turn:one",
      status: "running",
      terminalAt: null,
    }),
  });
  const fetchImpl = async (url, options) => {
    if (url.includes("/intents?")) {
      return json({ schemaVersion: 1, hostId: "host:one", cursor: 7, intents: [{ sequence: 7, intent: remoteIntent }] });
    }
    if (url.endsWith("/receipts")) {
      receipts.push(options.body);
      receiptAttempts += 1;
      if (receiptAttempts === 1) throw new Error("response dropped");
    }
    return json({ schemaVersion: 1, accepted: true });
  };
  const applyIntent = (candidate, options) => applyWorkControlCommand(candidate, {
    ...options,
    now: () => THIRD,
    ownerEpoch: "cloud-test-owner",
    cancel: async () => { cancelCalls += 1; },
  });

  assert.deepEqual(await syncCloud({ dataDir, fetchImpl, applyIntent, loadSnapshot: async () => snapshot(), now: () => THIRD }), {
    kind: "failed",
    code: "CLOUD_UNAVAILABLE",
  });
  assert.equal((await readCloudState({ dataDir })).connection.cursor, 0);

  assert.deepEqual(await syncCloud({ dataDir, fetchImpl, applyIntent, loadSnapshot: async () => snapshot(), now: () => THIRD }), {
    kind: "synced",
    cursor: 7,
    revision: 2,
  });
  assert.equal(cancelCalls, 1);
  assert.equal(receipts.length, 2);
  assert.equal(receipts[0], receipts[1]);
  assert.match(JSON.parse(receipts[1]).messageId, /^receipt:[0-9a-f]{64}$/);
  assert.equal((await readCloudState({ dataDir })).connection.cursor, 7);
});

test("an expired remote intent produces a durable expired receipt without provider control", async () => {
  const { dataDir } = await pairedDirectory();
  let cancelCalls = 0;
  let delivered;
  const expired = intent({ expiresAt: SECOND });
  const result = await syncCloud({
    dataDir,
    now: () => THIRD,
    loadSnapshot: async () => snapshot(),
    applyIntent: (candidate, options) => applyWorkControlCommand(candidate, {
      ...options,
      now: () => THIRD,
      cancel: async () => { cancelCalls += 1; },
    }),
    fetchImpl: async (url, options) => {
      if (url.includes("/intents?")) {
        return json({ schemaVersion: 1, hostId: "host:one", cursor: 4, intents: [{ sequence: 4, intent: expired }] });
      }
      if (url.endsWith("/receipts")) delivered = JSON.parse(options.body).payload.receipts[0];
      return json({ schemaVersion: 1, accepted: true });
    },
  });
  assert.equal(result.kind, "synced");
  assert.deepEqual([delivered.outcome, delivered.reasonCode, cancelCalls], ["expired", "COMMAND_EXPIRED", 0]);
});

test("Cloud outage is contained and does not mutate the durable cursor", async () => {
  const { dataDir } = await pairedDirectory();
  const result = await syncCloud({
    dataDir,
    loadSnapshot: async () => snapshot(),
    fetchImpl: async () => { throw new Error("offline"); },
    now: () => THIRD,
  });
  assert.deepEqual(result, { kind: "failed", code: "CLOUD_UNAVAILABLE" });
  const connection = (await readCloudState({ dataDir })).connection;
  assert.equal(connection.cursor, 0);
  assert.equal(connection.lastErrorCode, "CLOUD_UNAVAILABLE");
});
