import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { drainGeminiInbox } from "../lib/runtime/gemini-inbox.mjs";
import {
  persistGeminiLifecycleSignal,
  persistObservation,
  readProjection,
  replayEvents,
} from "../lib/runtime/observation-store.mjs";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const HOOK = join(ROOT, "extensions", "patchfleet-gemini", "hook.mjs");
const NOW = "2026-07-16T12:00:00.000Z";
const CANARY = "CANARY_PROMPT_RESPONSE_CWD_TRANSCRIPT_TOOL_TOKEN_ENV_CREDENTIAL_PATH";

function runHook(input, dataDir) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [HOOK], {
      cwd: ROOT,
      env: { ...process.env, PATCHFLEET_DATA_DIR: dataDir },
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, stdout, stderr }));
    child.stdin.end(input);
  });
}

test("native extension registers three bounded lifecycle hooks", async () => {
  const manifest = JSON.parse(await readFile(join(dirname(HOOK), "gemini-extension.json"), "utf8"));
  assert.deepEqual(manifest, {
    name: "patchfleet-gemini",
    version: "0.1.0",
    description: "Sends sanitized Gemini CLI lifecycle signals to Patchfleet.",
  });

  const config = JSON.parse(await readFile(join(dirname(HOOK), "hooks", "hooks.json"), "utf8"));
  assert.deepEqual(Object.keys(config.hooks), ["SessionStart", "BeforeAgent", "AfterAgent"]);
  for (const definitions of Object.values(config.hooks)) {
    const hook = definitions[0].hooks[0];
    assert.equal(hook.type, "command");
    assert.equal(hook.command, 'node "${extensionPath}${/}hook.mjs"');
    assert.equal(hook.timeout, 5_000);
  }
});

test("hook is fail-open and inbox keeps only the validated signal", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-inbox-"));
  const payload = {
    session_id: "gemini-session-1",
    hook_event_name: "AfterAgent",
    timestamp: NOW,
    prompt: CANARY,
    response: CANARY,
    cwd: CANARY,
    transcript_path: CANARY,
    tool_input: CANARY,
    token: CANARY,
    environment: CANARY,
    credential: CANARY,
  };
  assert.deepEqual(await runHook(JSON.stringify(payload), dataDir), {
    code: 0,
    stdout: "{}",
    stderr: "",
  });

  const inbox = join(dataDir, "gemini-inbox");
  const [name] = await readdir(inbox);
  const stored = await readFile(join(inbox, name), "utf8");
  assert.deepEqual(JSON.parse(stored), {
    schemaVersion: 1,
    providerId: "gemini",
    providerSessionId: "gemini-session-1",
    status: "completed",
    observedAt: NOW,
  });
  assert.equal(stored.includes(CANARY), false);
  if (process.platform !== "win32") {
    assert.equal((await stat(inbox)).mode & 0o777, 0o700);
    assert.equal((await stat(join(inbox, name))).mode & 0o777, 0o600);
  }

  const invalid = join(inbox, "signal-invalid.json");
  const unknown = join(inbox, "keep.txt");
  await writeFile(invalid, `${CANARY}\n`, { mode: 0o600 });
  await writeFile(unknown, CANARY, { mode: 0o600 });
  await drainGeminiInbox(async () => { throw new Error(CANARY); }, { dataDir });
  assert.deepEqual((await readdir(inbox)).sort(), ["keep.txt", name].sort());

  await persistObservation({
    schemaVersion: 1,
    provider: {
      id: "gemini",
      displayName: "Gemini CLI",
      state: "available",
      version: "0.43.0",
      capabilities: { recentObservation: true, explicitLiveStatus: true },
    },
    observedAt: NOW,
    sessions: [],
  }, { dataDir });
  await drainGeminiInbox(
    (signal) => persistGeminiLifecycleSignal(signal, { dataDir }),
    { dataDir },
  );
  assert.deepEqual(await readdir(inbox), ["keep.txt"]);
  assert.deepEqual((await readProjection({ dataDir })).observations[0].sessions[0], {
    providerSessionId: "gemini-session-1",
    status: "completed",
    createdAt: null,
    lastObservedAt: NOW,
  });
  assert.equal((await replayEvents({ dataDir })).some((item) => item.type === "session.terminal"), false);
  const durable = `${await readFile(join(dataDir, "events.jsonl"), "utf8")}${await readFile(
    join(dataDir, "observation.json"),
    "utf8",
  )}`;
  assert.equal(durable.includes(CANARY), false);

  assert.deepEqual(await runHook(`${JSON.stringify(payload)}${CANARY}`, dataDir), {
    code: 0,
    stdout: "{}",
    stderr: "",
  });
  assert.deepEqual(await runHook("x".repeat(16_385) + CANARY, dataDir), {
    code: 0,
    stdout: "{}",
    stderr: "",
  });
  assert.deepEqual(await readdir(inbox), ["keep.txt"]);
});
