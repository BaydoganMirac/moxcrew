import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { observeClaude, probeClaude } from "../lib/providers/claude.mjs";
import { assertProviderObservation } from "./support/provider-observation-conformance.mjs";
import fakeCli from "./support/fake-cli.cjs";

const { writeFakeCli } = fakeCli;

const NOW = new Date("2026-07-16T12:00:00.000Z");
const TEST_TIMEOUT = 5_000;
const NUMERIC_STARTED_AT = 1_784_196_000_000;
const CLAUDE_PROVIDER = Object.freeze({ id: "claude", displayName: "Claude Code" });
const CLAUDE_ERRORS = Object.freeze({
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
const CANARY = "MUST_NOT_SURVIVE";

async function fakeClaude(mode, payload = []) {
  const directory = await mkdtemp(join(tmpdir(), "patchfleet-claude-"));
  const baseCommand = join(directory, "claude");
  const marker = join(directory, "marker");
  const source = `#!${process.execPath}
const fs = require("node:fs");
const mode = ${JSON.stringify(mode)};
const marker = ${JSON.stringify(marker)};
const payload = ${JSON.stringify(payload)};
if (process.argv[2] === "--version") {
  if (mode === "probe-timeout") setInterval(() => {}, 1000);
  else if (mode === "probe-failed") process.exit(2);
  else if (mode === "bad-version") console.log("Claude Code current");
  else if (mode === "old-version") console.log("2.1.168 (Claude Code)");
  else if (mode === "build-version") console.log("2.1.170+native (Claude Code)");
  else console.log("2.1.170 (Claude Code)");
} else {
  fs.appendFileSync(marker, process.argv.slice(2).join(" ") + "\\n");
  if (mode === "snapshot-timeout") {
    process.on("SIGTERM", () => fs.appendFileSync(marker, "term\\n"));
    setInterval(() => {}, 1000);
  } else if (mode === "snapshot-failed") {
    console.error(${JSON.stringify(CANARY)});
    process.exit(2);
  } else if (mode === "invalid-json") {
    process.stdout.write("not-json");
  } else if (mode === "too-large") {
    process.stdout.write("x".repeat(300000));
  } else {
    process.stdout.write(JSON.stringify(payload));
  }
}
`;
  const command = await writeFakeCli(baseCommand, source);
  return { command, marker };
}

function background(id, state, startedAt = "2026-07-16T10:00:00.000Z") {
  return {
    id,
    state,
    startedAt,
    cwd: CANARY,
    prompt: CANARY,
    summary: CANARY,
    transcript: CANARY,
    token: CANARY,
    toolOutput: CANARY,
    environment: { CANARY },
    nativePayload: CANARY,
  };
}

test("probe is bounded and accepts only the supported semantic-version surface", async (t) => {
  const missing = await probeClaude({ command: `missing-claude-${process.pid}`, timeoutMs: 100 });
  assert.deepEqual(missing, { state: "unavailable", code: "CLAUDE_NOT_FOUND", version: null });

  for (const [mode, code, version, timeoutMs] of [
    ["probe-timeout", "CLAUDE_PROBE_TIMEOUT", null, 500],
    ["probe-failed", "CLAUDE_PROBE_FAILED", null, TEST_TIMEOUT],
    ["bad-version", "CLAUDE_VERSION_MALFORMED", null, TEST_TIMEOUT],
    ["old-version", "CLAUDE_SURFACE_UNSUPPORTED", "2.1.168", TEST_TIMEOUT],
  ]) {
    await t.test(mode, async () => {
      const { command } = await fakeClaude(mode);
      assert.deepEqual(await probeClaude({ command, timeoutMs }), { state: "degraded", code, version });
    });
  }

  const build = await fakeClaude("build-version");
  assert.deepEqual(await probeClaude({ command: build.command, timeoutMs: TEST_TIMEOUT }), {
    state: "available",
    version: "2.1.170+native",
  });
});

test("Agent View maps only documented lifecycle and strips native fields", async () => {
  const payload = [
    background("working", "working"),
    background("done", "done"),
    background("failed", "failed"),
    background("stopped", "stopped"),
    background("blocked", "blocked", NUMERIC_STARTED_AT),
    { sessionId: "running", status: "running", startedAt: "2026-07-16T09:00:00.000Z", prompt: CANARY },
    { sessionId: "waiting", status: "waiting", startedAt: "2026-07-16T09:00:00.000Z", summary: CANARY },
    { sessionId: "idle", status: "idle", startedAt: "2026-07-16T09:00:00.000Z", cwd: CANARY },
  ];
  const { command, marker } = await fakeClaude("success", payload);
  const result = await observeClaude({ command, timeoutMs: TEST_TIMEOUT, now: () => NOW });

  assert.equal(result.provider.state, "available");
  assertProviderObservation(result, CLAUDE_PROVIDER, CLAUDE_ERRORS);
  assert.deepEqual(Object.fromEntries(result.sessions.map((item) => [item.providerSessionId, item.status])), {
    "job:blocked": "unknown",
    "job:done": "completed",
    "job:failed": "failed",
    "job:stopped": "interrupted",
    "job:working": "running",
    "session:idle": "unknown",
    "session:running": "running",
    "session:waiting": "unknown",
  });
  assert(result.sessions.every((session) => !Object.hasOwn(session, "terminalAt")));
  assert.equal(result.sessions.find((session) => session.providerSessionId === "job:blocked").createdAt, "2026-07-16T10:00:00.000Z");
  assert.equal(JSON.stringify(result).includes(CANARY), false);
  assert.equal(await readFile(marker, "utf8"), "agents --json --all\n");
});

test("empty, degraded, and unavailable observations pass shared conformance", async () => {
  const { command } = await fakeClaude("success");
  const available = await observeClaude({ command, timeoutMs: TEST_TIMEOUT, now: () => NOW });
  assert.deepEqual(available.sessions, []);
  assertProviderObservation(available, CLAUDE_PROVIDER, CLAUDE_ERRORS);

  const malformed = await fakeClaude("success", [background("job", "undocumented")]);
  const degraded = await observeClaude({ command: malformed.command, timeoutMs: TEST_TIMEOUT, now: () => NOW });
  assert.equal(degraded.provider.state, "degraded");
  assert.equal(degraded.provider.error.code, "CLAUDE_SNAPSHOT_UNSUPPORTED");
  assertProviderObservation(degraded, CLAUDE_PROVIDER, CLAUDE_ERRORS);

  const unavailable = await observeClaude({
    command: `missing-claude-${process.pid}`,
    timeoutMs: 100,
    now: () => NOW,
  });
  assert.equal(unavailable.provider.state, "unavailable");
  assertProviderObservation(unavailable, CLAUDE_PROVIDER, CLAUDE_ERRORS);
});

test("Agent View rejects malformed snapshots instead of inventing identity or lifecycle", async (t) => {
  const cases = [
    ["missing id", [{ state: "working", startedAt: NOW.toISOString() }]],
    ["empty background id", [background("", "working")]],
    ["empty live session id", [{ sessionId: "", status: "running", startedAt: NOW.toISOString() }]],
    ["duplicate id", [background("same", "working"), background("same", "done")]],
    ["oversized id", [background("a".repeat(253), "working")]],
    ["invalid id", [background("invalid id", "working")]],
    ["invalid timestamp", [background("job", "working", "not-a-time")]],
    ["noncanonical timestamp", [background("job", "working", "2026-07-16T10:00:00Z")]],
    ["numeric string timestamp", [background("job", "working", String(NUMERIC_STARTED_AT))]],
    ["seconds timestamp", [background("job", "working", 1_752_661_800)]],
    ["fractional timestamp", [background("job", "working", NUMERIC_STARTED_AT + 0.5)]],
    ["short millisecond timestamp", [background("job", "working", 999_999_999_999)]],
    ["long millisecond timestamp", [background("job", "working", 10_000_000_000_000)]],
    ["non-finite timestamp", [background("job", "working", Number.POSITIVE_INFINITY)]],
    ["missing state", [{ id: "job", startedAt: NOW.toISOString() }]],
    ["unsupported live status", [{ sessionId: "live", status: "done", startedAt: NOW.toISOString() }]],
    ["non-object entry", [null]],
    ["non-array snapshot", { sessions: [] }],
  ];
  for (const [name, payload] of cases) {
    await t.test(name, async () => {
      const { command } = await fakeClaude("success", payload);
      const result = await observeClaude({ command, timeoutMs: TEST_TIMEOUT, now: () => NOW });
      assert.equal(result.provider.error.code, "CLAUDE_SNAPSHOT_UNSUPPORTED");
      assert.deepEqual(result.sessions, []);
      assert.equal(JSON.stringify(result).includes(CANARY), false);
    });
  }
});

test("Agent View failures stay bounded, safe, and clean up timed-out children", async (t) => {
  for (const [mode, code] of [
    ["snapshot-timeout", "CLAUDE_SNAPSHOT_TIMEOUT"],
    ["snapshot-failed", "CLAUDE_SNAPSHOT_FAILED"],
    ["invalid-json", "CLAUDE_SNAPSHOT_MALFORMED"],
    ["too-large", "CLAUDE_SNAPSHOT_FAILED"],
  ]) {
    await t.test(mode, async () => {
      const { command, marker } = await fakeClaude(mode);
      const result = await observeClaude({ command, timeoutMs: TEST_TIMEOUT, now: () => NOW });
      assert.equal(result.provider.error.code, code);
      assert.equal(JSON.stringify(result).includes(CANARY), false);
      if (mode === "snapshot-timeout") {
        assert.equal(
          await readFile(marker, "utf8"),
          process.platform === "win32" ? "agents --json --all\n" : "agents --json --all\nterm\n",
        );
      }
    });
  }
});

test("Agent View prefers non-terminal sessions, orders deterministically, and caps at 20", async () => {
  const payload = [];
  for (let index = 0; index < 12; index++) {
    payload.push(background(`terminal-${String(index).padStart(2, "0")}`, "done", `2026-07-16T11:${String(index).padStart(2, "0")}:00.000Z`));
    payload.push(background(`running-${String(index).padStart(2, "0")}`, "working", `2026-07-16T09:${String(index).padStart(2, "0")}:00.000Z`));
  }
  payload.push(background("tie-b", "working", "2026-07-16T10:30:00.000Z"));
  payload.push(background("tie-a", "working", "2026-07-16T10:30:00.000Z"));
  const { command } = await fakeClaude("success", payload.reverse());
  const result = await observeClaude({ command, timeoutMs: TEST_TIMEOUT, now: () => NOW });

  assert.equal(result.sessions.length, 20);
  assert(result.sessions.slice(0, 14).every((session) => session.status === "running"));
  assert.deepEqual(result.sessions.slice(0, 2).map((session) => session.providerSessionId), ["job:tie-a", "job:tie-b"]);
  assert.deepEqual(
    result.sessions.slice(14).map((session) => session.providerSessionId),
    ["job:terminal-11", "job:terminal-10", "job:terminal-09", "job:terminal-08", "job:terminal-07", "job:terminal-06"],
  );
});
