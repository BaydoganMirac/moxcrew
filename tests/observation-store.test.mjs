import assert from "node:assert/strict";
import { appendFile, mkdtemp, readFile, readdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  normalizeObservation,
  safeObservationError,
  validateProjection,
} from "../lib/domain/observation.mjs";
import {
  appendEvents,
  persistGeminiLifecycleSignal,
  persistObservation,
  projectEvents,
  readProjection,
  rebuildProjection,
  replayEvents,
  StorageCorruptionError,
} from "../lib/runtime/observation-store.mjs";

const FIRST = "2026-07-15T12:00:00.000Z";
const SECOND = "2026-07-15T12:05:00.000Z";
const THIRD = "2026-07-15T12:10:00.000Z";
const FOURTH = "2026-07-15T12:15:00.000Z";
const PROVIDERS = {
  codex: "Codex",
  claude: "Claude Code",
  gemini: "Gemini CLI",
};

function observation({
  providerId = "codex",
  observedAt = FIRST,
  status = "completed",
  terminalAt = "2026-07-15T11:59:00.000Z",
  state = "available",
  error,
  sessions,
  extra = {},
} = {}) {
  return {
    schemaVersion: 1,
    provider: {
      id: providerId,
      displayName: PROVIDERS[providerId],
      state,
      version: "1.2.3",
      capabilities: {
        recentObservation: state === "available",
        explicitLiveStatus: state === "available",
      },
      ...(error ? { error } : {}),
      ...extra,
    },
    observedAt,
    sessions: sessions ?? [
      {
        providerSessionId: "session-1",
        status,
        createdAt: "2026-07-15T11:00:00.000Z",
        lastObservedAt: observedAt,
        ...(terminalAt ? { terminalAt } : {}),
        ...extra,
      },
    ],
    ...extra,
  };
}

function provider(projection, providerId = "codex") {
  return projection.observations.find((item) => item.provider.id === providerId);
}

async function directory() {
  return mkdtemp(join(tmpdir(), "patchfleet-store-"));
}

test("append, replay, deterministic rebuild, and atomic projection replacement", async () => {
  const dataDir = await directory();
  const first = await persistObservation(observation(), { dataDir });
  const events = await replayEvents({ dataDir });
  assert.ok(events.length >= 3);
  assert.deepEqual(projectEvents(events), first);
  assert.deepEqual(await readProjection({ dataDir }), first);

  await writeFile(join(dataDir, "observation.json"), "broken", "utf8");
  assert.deepEqual(await rebuildProjection({ dataDir }), first);

  const second = await persistObservation(
    observation({ observedAt: SECOND, status: "running", terminalAt: undefined }),
    { dataDir },
  );
  assert.equal(provider(second).observedAt, SECOND);
  assert.equal(provider(await readProjection({ dataDir })).sessions[0].status, "running");
  assert.deepEqual((await readdir(dataDir)).filter((name) => name.endsWith(".tmp")), []);
});

test("duplicate event ids are rejected", async () => {
  const dataDir = await directory();
  await persistObservation(observation(), { dataDir });
  const [event] = await replayEvents({ dataDir });
  await assert.rejects(() => appendEvents([event], { dataDir }), StorageCorruptionError);
});

test("only an incomplete final line is ignored and repaired", async () => {
  const dataDir = await directory();
  await persistObservation(observation(), { dataDir });
  const before = await replayEvents({ dataDir });
  await appendFile(join(dataDir, "events.jsonl"), '{"partial":', "utf8");
  assert.deepEqual(await replayEvents({ dataDir }), before);

  await persistObservation(observation({ observedAt: SECOND }), { dataDir });
  const repaired = await readFile(join(dataDir, "events.jsonl"), "utf8");
  assert.equal(repaired.includes('"partial"'), false);
  assert.doesNotThrow(() => repaired.trimEnd().split("\n").forEach(JSON.parse));
});

test("middle corruption fails closed", async () => {
  const dataDir = await directory();
  await persistObservation(observation(), { dataDir });
  const log = await readFile(join(dataDir, "events.jsonl"), "utf8");
  const lines = log.trimEnd().split("\n");
  lines.splice(1, 0, "not-json");
  await writeFile(join(dataDir, "events.jsonl"), `${lines.join("\n")}\n`, "utf8");
  await assert.rejects(() => replayEvents({ dataDir }), StorageCorruptionError);
  await assert.rejects(() => persistObservation(observation({ observedAt: SECOND }), { dataDir }), StorageCorruptionError);
});

test("only consecutive identical terminal observations are deduplicated", async () => {
  const dataDir = await directory();
  await persistObservation(observation(), { dataDir });
  await persistObservation(observation({ providerId: "claude" }), { dataDir });
  await persistObservation(observation({ observedAt: SECOND }), { dataDir });
  assert.equal(
    (await replayEvents({ dataDir })).filter((event) => event.type === "session.terminal").length,
    2,
  );

  await persistObservation(observation({ observedAt: THIRD, status: "running", terminalAt: null }), {
    dataDir,
  });
  await persistObservation(observation({ observedAt: FOURTH }), { dataDir });
  const events = await replayEvents({ dataDir });
  assert.equal(events.filter((event) => event.type === "session.terminal").length, 3);
  assert.equal(events.filter((event) => event.type === "session.observed").length, 5);
});

test("fixed providers normalize with provider-scoped safe errors", () => {
  const combinedVersion = observation({ providerId: "claude" });
  combinedVersion.provider.version = "2.1.169-beta.1+build.7";
  assert.equal(normalizeObservation(combinedVersion).provider.version, "2.1.169-beta.1+build.7");

  for (const [providerId, code] of [
    ["codex", "CODEX_NOT_FOUND"],
    ["claude", "CLAUDE_NOT_FOUND"],
    ["gemini", "GEMINI_HOOK_SETUP_REQUIRED"],
  ]) {
    const error = safeObservationError(providerId, code);
    const normalized = normalizeObservation(observation({
      providerId,
      state: providerId === "codex" || providerId === "claude" ? "unavailable" : "degraded",
      error: { code, message: "CANARY_UNSAFE_ERROR" },
      sessions: [],
    }));
    assert.deepEqual(normalized.provider.error, error);
    assert.equal(normalized.provider.displayName, PROVIDERS[providerId]);
  }

  assert.throws(() => safeObservationError("claude", "CODEX_NOT_FOUND"));
  assert.throws(() => normalizeObservation({
    ...observation(),
    provider: { ...observation().provider, displayName: "CANARY_PROVIDER" },
  }));
});

test("provider sessions, failures, replay, and ordering remain isolated", async () => {
  const dataDir = await directory();
  await persistObservation(observation(), { dataDir });
  await persistObservation(observation({
    providerId: "claude",
    observedAt: SECOND,
    status: "running",
    terminalAt: null,
  }), { dataDir });
  const projection = await persistObservation(observation({
    providerId: "gemini",
    observedAt: THIRD,
    state: "degraded",
    error: safeObservationError("gemini", "GEMINI_HOOK_SETUP_REQUIRED"),
    sessions: [],
  }), { dataDir });

  assert.deepEqual(
    projection.observations.map((item) => item.provider.id),
    ["codex", "claude", "gemini"],
  );
  assert.equal(provider(projection, "codex").sessions[0].status, "completed");
  assert.equal(provider(projection, "claude").sessions[0].status, "running");
  assert.equal(provider(projection, "gemini").provider.error.code, "GEMINI_HOOK_SETUP_REQUIRED");
  assert.deepEqual(projectEvents(await replayEvents({ dataDir })), projection);
  assert.deepEqual(await rebuildProjection({ dataDir }), projection);
  assert.deepEqual(await readProjection({ dataDir }), projection);

  const isolated = await persistObservation(observation({
    providerId: "claude",
    observedAt: FOURTH,
    state: "unavailable",
    error: safeObservationError("claude", "CLAUDE_NOT_FOUND"),
    sessions: [],
  }), { dataDir });
  assert.equal(provider(isolated, "codex").sessions[0].status, "completed");
  assert.equal(provider(isolated, "claude").sessions.length, 0);
});

test("legacy Codex projection is wrapped and duplicate providers are rejected", async () => {
  const dataDir = await directory();
  const legacy = normalizeObservation(observation());
  await writeFile(join(dataDir, "observation.json"), `${JSON.stringify(legacy)}\n`, "utf8");
  assert.deepEqual(await readProjection({ dataDir }), {
    schemaVersion: 2,
    observations: [legacy],
  });
  assert.throws(() => validateProjection({
    schemaVersion: 2,
    observations: [legacy, legacy],
  }));
});

test("forbidden extra fields never enter the event log or projection", async () => {
  const dataDir = await directory();
  const canary = "CANARY_PROMPT_TRANSCRIPT_PATH_TOOL_TOKEN";
  await persistObservation(observation({ extra: { prompt: canary, cwd: canary, transcript: canary } }), {
    dataDir,
  });
  await persistObservation(observation({
    providerId: "claude",
    extra: { prompt: canary, cwd: canary, transcript: canary },
  }), { dataDir });
  const stored = `${await readFile(join(dataDir, "events.jsonl"), "utf8")}${await readFile(
    join(dataDir, "observation.json"),
    "utf8",
  )}`;
  assert.equal(stored.includes(canary), false);
  assert.equal(stored.includes("prompt"), false);
  assert.equal(stored.includes("transcript"), false);
  assert.equal(stored.includes("cwd"), false);
});

test("Gemini lifecycle signals are idempotent, non-terminal, retained, and setup-scoped", async () => {
  const dataDir = await directory();
  await persistObservation(observation({ status: "running", terminalAt: null }), { dataDir });
  await persistObservation(observation({
    providerId: "gemini",
    observedAt: SECOND,
    sessions: [],
  }), { dataDir });

  for (const [status, observedAt] of [
    ["unknown", SECOND],
    ["running", THIRD],
    ["completed", FOURTH],
  ]) {
    await persistGeminiLifecycleSignal({
      schemaVersion: 1,
      providerId: "gemini",
      providerSessionId: "gemini-hook-session",
      status,
      observedAt,
    }, { dataDir });
  }

  const beforeRetry = await replayEvents({ dataDir });
  await persistGeminiLifecycleSignal({
    schemaVersion: 1,
    providerId: "gemini",
    providerSessionId: "gemini-hook-session",
    status: "completed",
    observedAt: FOURTH,
  }, { dataDir });
  assert.equal((await replayEvents({ dataDir })).length, beforeRetry.length);

  let projection = await readProjection({ dataDir });
  const hookSession = provider(projection, "gemini").sessions[0];
  assert.deepEqual(hookSession, {
    providerSessionId: "gemini-hook-session",
    status: "completed",
    createdAt: null,
    lastObservedAt: FOURTH,
  });
  assert.equal(
    beforeRetry.some((item) => item.type === "session.terminal" && item.payload.providerId === "gemini"),
    false,
  );

  for (let index = 0; index < 21; index += 1) {
    await persistGeminiLifecycleSignal({
      schemaVersion: 1,
      providerId: "gemini",
      providerSessionId: `retained-${String(index).padStart(2, "0")}`,
      status: "running",
      observedAt: new Date(Date.parse(FOURTH) + (index + 1) * 1_000).toISOString(),
    }, { dataDir });
  }
  projection = await readProjection({ dataDir });
  assert.equal(provider(projection, "gemini").sessions.length, 20);
  assert.equal(provider(projection, "gemini").sessions.some((item) => item.providerSessionId === "retained-20"), true);
  assert.equal(provider(projection, "gemini").sessions.some((item) => item.providerSessionId === "gemini-hook-session"), false);

  const sessionEventsBeforeRefresh = (await replayEvents({ dataDir })).filter(
    (item) => item.type === "session.observed" && item.payload.providerId === "gemini",
  ).length;
  projection = await persistObservation(observation({
    providerId: "gemini",
    observedAt: new Date(Date.parse(FOURTH) + 30_000).toISOString(),
    sessions: [],
  }), { dataDir, preserveSessions: true });
  assert.equal(provider(projection, "gemini").sessions.length, 20);
  assert.equal(
    (await replayEvents({ dataDir })).filter(
      (item) => item.type === "session.observed" && item.payload.providerId === "gemini",
    ).length,
    sessionEventsBeforeRefresh,
  );

  projection = await persistObservation(observation({
    providerId: "gemini",
    observedAt: new Date(Date.parse(FOURTH) + 31_000).toISOString(),
    state: "degraded",
    error: safeObservationError("gemini", "GEMINI_PROBE_FAILED"),
    sessions: [],
  }), { dataDir, preserveSessions: true });
  assert.equal(provider(projection, "gemini").sessions.length, 20);
  assert.equal(
    (await replayEvents({ dataDir })).filter(
      (item) => item.type === "session.observed" && item.payload.providerId === "gemini",
    ).length,
    sessionEventsBeforeRefresh,
  );

  projection = await persistObservation(observation({
    providerId: "gemini",
    observedAt: new Date(Date.parse(FOURTH) + 32_000).toISOString(),
    state: "degraded",
    error: safeObservationError("gemini", "GEMINI_HOOK_SETUP_REQUIRED"),
    sessions: [],
  }), { dataDir });
  assert.deepEqual(provider(projection, "gemini").sessions, []);
  assert.equal(provider(projection, "codex").sessions[0].status, "running");
});
