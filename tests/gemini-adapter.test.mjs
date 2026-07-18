import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import {
  decodeGeminiHook,
  observeGemini,
  probeGemini,
  probeGeminiExtension,
} from "../lib/providers/gemini.mjs";
import { validateProviderLifecycleSignal } from "../lib/domain/provider-lifecycle-signal.mjs";
import { assertProviderObservation } from "./support/provider-observation-conformance.mjs";
import fakeCli from "./support/fake-cli.cjs";

const { writeFakeCli } = fakeCli;

const NOW = new Date("2026-07-16T12:00:00.000Z");
const GEMINI_PROVIDER = Object.freeze({ id: "gemini", displayName: "Gemini CLI" });
const GEMINI_ERRORS = Object.freeze({
  GEMINI_HOOK_SETUP_REQUIRED: "Gemini CLI hook setup is required.",
  GEMINI_NOT_FOUND: "Gemini CLI is not installed or is not on PATH.",
  GEMINI_PROBE_FAILED: "Gemini CLI could not be checked.",
  GEMINI_PROBE_TIMEOUT: "Gemini CLI version check timed out.",
  GEMINI_VERSION_MALFORMED: "Gemini CLI returned an unsupported version.",
});
const CANARY = "CANARY_SECRET_PAYLOAD";

async function fakeGemini(mode, extensionOutput = "[]") {
  const directory = await mkdtemp(join(tmpdir(), "patchfleet-gemini-"));
  const baseCommand = join(directory, "gemini");
  const marker = join(directory, "argv");
  const source = `#!/usr/bin/env node
const fs = require("node:fs");
const args = process.argv.slice(2);
fs.appendFileSync(${JSON.stringify(marker)}, JSON.stringify(args) + "\\n");
if (args[0] === "--version") {
  if (${JSON.stringify(mode)} === "timeout") setInterval(() => {}, 1000);
  else if (${JSON.stringify(mode)} === "failed") process.exit(2);
  else if (${JSON.stringify(mode)} === "malformed") console.log("Gemini CLI version unknown");
  else console.log("0.43.0");
} else if (${JSON.stringify(mode)} === "extension-timeout") setInterval(() => {}, 1000);
else if (${JSON.stringify(mode)} === "extension-failed") process.exit(2);
else if (${JSON.stringify(mode)} === "extension-malformed") console.log("{");
else if (${JSON.stringify(mode)} === "extension-stderr") console.error(${JSON.stringify(extensionOutput)});
else if (${JSON.stringify(mode)} !== "extension-blank") console.log(${JSON.stringify(extensionOutput)});
`;
  const command = await writeFakeCli(baseCommand, source);
  return { command, marker };
}

test("probe uses only a bounded version command", async (t) => {
  const missing = await probeGemini({ command: `missing-gemini-${process.pid}`, timeoutMs: 100 });
  assert.equal(missing.code, "GEMINI_NOT_FOUND");

  for (const [mode, code] of [
    ["timeout", "GEMINI_PROBE_TIMEOUT"],
    ["failed", "GEMINI_PROBE_FAILED"],
    ["malformed", "GEMINI_VERSION_MALFORMED"],
    ["valid", undefined],
  ]) {
    await t.test(mode, async () => {
      const { command, marker } = await fakeGemini(mode);
      const result = await probeGemini({ command, timeoutMs: mode === "timeout" ? 500 : 2_000 });
      assert.equal(result.code, code);
      if (mode === "valid") assert.deepEqual(result, { state: "available", version: "0.43.0" });
      if (mode !== "timeout") assert.equal(await readFile(marker, "utf8"), '["--version"]\n');
    });
  }
});

test("extension status probe is bounded and uses structured output", async (t) => {
  const missing = await probeGeminiExtension({
    command: `missing-gemini-${process.pid}`,
    timeoutMs: 100,
  });
  assert.deepEqual(missing, { state: "unavailable", code: "GEMINI_NOT_FOUND" });

  for (const [name, mode, output, expected] of [
    ["active", "valid", JSON.stringify([{ name: "patchfleet-gemini", isActive: true, path: CANARY }]), { state: "active" }],
    ["active-stderr", "extension-stderr", JSON.stringify([{ name: "patchfleet-gemini", isActive: true, path: CANARY }]), { state: "active" }],
    ["missing", "valid", JSON.stringify([{ name: "other", isActive: true }]), { state: "setup-required" }],
    ["inactive", "valid", JSON.stringify([{ name: "patchfleet-gemini", isActive: false }]), { state: "setup-required" }],
    ["empty", "valid", "[]", { state: "setup-required" }],
    ["blank", "extension-blank", "", { state: "setup-required" }],
    ["malformed", "extension-malformed", "", { state: "degraded", code: "GEMINI_PROBE_FAILED" }],
    ["failed", "extension-failed", "", { state: "degraded", code: "GEMINI_PROBE_FAILED" }],
    ["timeout", "extension-timeout", "", { state: "degraded", code: "GEMINI_PROBE_TIMEOUT" }],
  ]) {
    await t.test(name, async () => {
      const { command, marker } = await fakeGemini(mode, output);
      const result = await probeGeminiExtension({ command, timeoutMs: name === "timeout" ? 500 : 2_000 });
      assert.deepEqual(result, expected);
      assert.equal(JSON.stringify(result).includes(CANARY), false);
      if (name !== "timeout") {
        assert.equal(
          await readFile(marker, "utf8"),
          '["extensions","list","--output-format","json"]\n',
        );
      }
    });
  }
});

test("observation reports unavailable or explicit hook setup required", async () => {
  const unavailable = await observeGemini({
    command: `missing-gemini-${process.pid}`,
    timeoutMs: 100,
    now: () => NOW,
  });
  assertProviderObservation(unavailable, GEMINI_PROVIDER, GEMINI_ERRORS);
  assert.equal(unavailable.provider.state, "unavailable");

  const { command, marker } = await fakeGemini("valid");
  const setupRequired = await observeGemini({ command, timeoutMs: 2_000, now: () => NOW });
  assertProviderObservation(setupRequired, GEMINI_PROVIDER, GEMINI_ERRORS);
  assert.equal(setupRequired.provider.state, "degraded");
  assert.equal(setupRequired.provider.error.code, "GEMINI_HOOK_SETUP_REQUIRED");
  assert.deepEqual(setupRequired.provider.capabilities, {
    recentObservation: false,
    explicitLiveStatus: false,
  });
  assert.deepEqual(setupRequired.sessions, []);
  assert.equal(
    await readFile(marker, "utf8"),
    '["--version"]\n["extensions","list","--output-format","json"]\n',
  );

  const activeCommand = await fakeGemini("valid", JSON.stringify([
    { name: "patchfleet-gemini", isActive: true, path: CANARY, settings: CANARY },
  ]));
  const active = await observeGemini({ command: activeCommand.command, timeoutMs: 2_000, now: () => NOW });
  assertProviderObservation(active, GEMINI_PROVIDER, GEMINI_ERRORS);
  assert.equal(active.provider.state, "available");
  assert.deepEqual(active.provider.capabilities, {
    recentObservation: true,
    explicitLiveStatus: true,
  });
  assert.equal(JSON.stringify(active).includes(CANARY), false);
});

test("hook decoder maps only explicit lifecycle events", () => {
  const expected = {
    SessionStart: "unknown",
    BeforeAgent: "running",
    AfterAgent: "completed",
  };
  for (const [hook_event_name, status] of Object.entries(expected)) {
    const signal = decodeGeminiHook(JSON.stringify({
      session_id: "gemini-session-1",
      hook_event_name,
      timestamp: NOW.toISOString(),
    }));
    assert.equal(signal.status, status);
    assert.deepEqual(validateProviderLifecycleSignal(signal), signal);
  }

  for (const reason of ["exit", "error", "abort"]) {
    assert.equal(decodeGeminiHook(JSON.stringify({
      session_id: "gemini-session-1",
      hook_event_name: "SessionEnd",
      timestamp: NOW.toISOString(),
      reason,
    })), null);
  }
  for (const event of ["AfterAgentError", "AgentAbort", "failed", "interrupted"]) {
    assert.throws(() => decodeGeminiHook(JSON.stringify({
      session_id: "gemini-session-1",
      hook_event_name: event,
      timestamp: NOW.toISOString(),
    })), /unsupported Gemini hook event/);
  }
});

test("hook decoder rejects malformed or oversized input safely", () => {
  const valid = {
    session_id: "gemini-session-1",
    hook_event_name: "BeforeAgent",
    timestamp: NOW.toISOString(),
  };
  const cases = [
    "not-json",
    "null",
    "[]",
    JSON.stringify({ ...valid, session_id: "invalid id" }),
    JSON.stringify({ ...valid, session_id: "x".repeat(257) }),
    JSON.stringify({ ...valid, timestamp: "not-a-timestamp" }),
    JSON.stringify({ ...valid, hook_event_name: "Unsupported" }),
    JSON.stringify({ ...valid, padding: "x".repeat(16_384) }),
  ];
  for (const field of ["session_id", "hook_event_name", "timestamp"]) {
    const missing = { ...valid };
    delete missing[field];
    cases.push(JSON.stringify(missing));
  }
  for (const input of cases) {
    assert.throws(() => decodeGeminiHook(input), TypeError);
  }
});

test("hook decoder discards forbidden native fields and canaries", () => {
  const payload = {
    session_id: "gemini-session-1",
    hook_event_name: "AfterAgent",
    timestamp: NOW.toISOString(),
  };
  for (const field of [
    "prompt",
    "response",
    "cwd",
    "transcript_path",
    "tool_input",
    "tool_output",
    "model",
    "token",
    "environment",
    "credential",
    "source",
    "reason",
    "native_payload",
  ]) payload[field] = `${CANARY}:${field}`;

  const signal = decodeGeminiHook(JSON.stringify(payload));
  assert.equal(JSON.stringify(signal).includes(CANARY), false);
  assert.deepEqual(Object.keys(signal), [
    "schemaVersion",
    "providerId",
    "providerSessionId",
    "status",
    "observedAt",
  ]);

  try {
    decodeGeminiHook(`${JSON.stringify(payload)}${CANARY}`);
    assert.fail("invalid payload should throw");
  } catch (error) {
    assert.equal(String(error).includes(CANARY), false);
  }
});
