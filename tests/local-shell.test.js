const assert = require("node:assert/strict");
const { spawn } = require("node:child_process");
const { once } = require("node:events");
const { mkdir, mkdtemp, readFile, writeFile } = require("node:fs/promises");
const { request } = require("node:http");
const { createServer } = require("node:net");
const { tmpdir } = require("node:os");
const { delimiter, join } = require("node:path");
const { test } = require("node:test");
const { writeFakeCli } = require("./support/fake-cli.cjs");

const requiredHeaders = {
  "permissions-policy": "camera=(), microphone=(), geolocation=()",
  "referrer-policy": "same-origin",
  "x-content-type-options": "nosniff",
  "x-frame-options": "DENY",
};

async function freePort() {
  const server = createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address();
  server.close();
  await once(server, "close");
  return port;
}

function fetch(port, { host = `127.0.0.1:${port}`, path = "/", method = "GET", headers = {}, setHost = true, body } = {}) {
  return new Promise((resolve, reject) => {
    const req = request(
      {
        host: "127.0.0.1",
        port,
        path,
        method,
        headers: { ...(host ? { host } : {}), ...headers },
        setHost,
      },
      (res) => {
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve({
          statusCode: res.statusCode,
          headers: res.headers,
          body: Buffer.concat(chunks).toString("utf8"),
        }));
      },
    );
    req.on("error", reject);
    req.end(body);
  });
}

async function waitForServer(port, child) {
  for (let attempt = 0; attempt < 150; attempt += 1) {
    if (child.exitCode !== null) throw new Error("Next.js exited before serving");
    try {
      await fetch(port);
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  throw new Error("Timed out waiting for Next.js");
}

async function startNext(dataDir, binDir) {
  const port = await freePort();
  const syncToken = `test-sync-${port}`;
  const production = process.env.PATCHFLEET_TEST_PRODUCTION === "1";
  const child = spawn(
    process.execPath,
    [
      require.resolve("next/dist/bin/next"),
      production ? "start" : "dev",
      ...(production ? [] : ["--turbopack"]),
      "--hostname",
      "127.0.0.1",
      "--port",
      String(port),
    ],
    {
      env: {
        ...process.env,
        PATCHFLEET_DATA_DIR: dataDir,
        PATCHFLEET_OWNER_EPOCH: `test-owner-${port}`,
        PATCHFLEET_SYNC_TOKEN: syncToken,
        PATH: `${binDir}${delimiter}${process.env.PATH}`,
      },
      stdio: "ignore",
    },
  );
  await waitForServer(port, child);
  return { child, port, syncToken };
}

async function stopNext(child) {
  if (child.exitCode !== null) return;
  const exited = once(child, "exit");
  child.kill("SIGTERM");
  await exited;
}

async function fakeProviders() {
  const binDir = await mkdtemp(join(tmpdir(), "patchfleet-web-bin-"));
  const codex = join(binDir, "codex");
  const claude = join(binDir, "claude");
  const gemini = join(binDir, "gemini");
  const geminiMode = join(binDir, "gemini-mode");
  const marker = join(binDir, "marker");
  const controlState = join(binDir, "control-state.json");
  const codexSource = `#!/usr/bin/env node
const fs = require("node:fs");
const readline = require("node:readline");
const marker = ${JSON.stringify(marker)};
const controlState = ${JSON.stringify(controlState)};
const readState = () => fs.existsSync(controlState)
  ? JSON.parse(fs.readFileSync(controlState, "utf8"))
  : { threadStarts: 0, turnStarts: 0, interrupts: 0, threads: [] };
const writeState = (state) => fs.writeFileSync(controlState, JSON.stringify(state));
if (process.argv.includes("--version")) console.log("codex-cli 1.2.3");
else {
  fs.appendFileSync(marker, "start\\n");
  process.on("exit", () => fs.appendFileSync(marker, "stop\\n"));
  process.on("SIGTERM", () => process.exit(0));
  const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
  readline.createInterface({ input: process.stdin }).on("line", (line) => {
    const message = JSON.parse(line);
    if (message.method === "initialized") return;
    if (message.method === "initialize") return send({ id: message.id, result: { codexHome: "/private/CANARY_PATH" } });
    if (message.method === "thread/list") {
      const state = readState();
      const data = JSON.stringify(message.params.sourceKinds) === JSON.stringify(["appServer"])
        ? state.threads
        : [{ id: "web-session-id", preview: "CANARY_PROMPT" }];
      return send({ id: message.id, result: { data, nextCursor: null } });
    }
    if (message.method === "thread/read") {
      const controlled = readState().threads.find((item) => item.id === message.params.threadId);
      return send({ id: message.id, result: { thread: controlled ?? {
        id: "web-session-id", createdAt: 1700000000, status: { type: "active", activeFlags: [] }, turns: [],
        preview: "CANARY_PROMPT", cwd: "/private/CANARY_PATH", items: [{ text: "CANARY_TRANSCRIPT" }]
      } } });
    }
    if (message.method === "thread/start") {
      const state = readState();
      const thread = { id: "web-control-thread", cwd: message.params.cwd, threadSource: message.params.threadSource, ephemeral: false, turns: [] };
      state.threadStarts += 1; state.threads = [thread]; writeState(state);
      return send({ id: message.id, result: { thread } });
    }
    if (message.method === "thread/resume") {
      const thread = readState().threads.find((item) => item.id === message.params.threadId);
      return send({ id: message.id, result: { thread } });
    }
    if (message.method === "turn/start") {
      const state = readState();
      state.turnStarts += 1;
      state.threads[0].turns = [{ id: "web-control-turn", status: "inProgress", completedAt: null, items: [{ text: "CANARY_TRANSCRIPT" }] }];
      writeState(state);
      return send({ id: message.id, result: { turn: state.threads[0].turns[0] } });
    }
    if (message.method === "turn/interrupt") {
      const state = readState();
      state.interrupts += 1;
      state.threads[0].turns[0].status = "interrupted";
      state.threads[0].turns[0].completedAt = 1784200000;
      writeState(state);
      return send({ id: message.id, result: {} });
    }
  });
}
`;
  const claudeSource = `#!/usr/bin/env node
if (process.argv.includes("--version")) console.log("2.1.170 (Claude Code)");
else console.log(JSON.stringify([{
  id: "claude-session-id", state: "working", startedAt: 1784196000000,
  prompt: "CLAUDE_CANARY_PROMPT", cwd: "/private/CLAUDE_CANARY_PATH"
}]));
`;
  const geminiSource = `#!/usr/bin/env node
const fs = require("node:fs");
if (process.argv.includes("--version")) console.log("0.43.0");
else if (process.argv[2] === "extensions") {
  const mode = fs.existsSync(${JSON.stringify(geminiMode)})
    ? fs.readFileSync(${JSON.stringify(geminiMode)}, "utf8").trim()
    : "missing";
  if (mode === "failed") process.exit(1);
  else console.log("[]");
}
else process.exit(1);
`;
  await writeFakeCli(codex, codexSource);
  await writeFakeCli(claude, claudeSource);
  await writeFakeCli(gemini, geminiSource);
  return { binDir, geminiMode, marker, controlState };
}

function formBody(fields) {
  return new URLSearchParams(fields).toString();
}

function postForm(port, fields, options = {}) {
  const body = typeof fields === "string" ? fields : formBody(fields);
  return fetch(port, {
    path: "/api/work",
    method: "POST",
    headers: {
      origin: `http://127.0.0.1:${port}`,
      "content-type": "application/x-www-form-urlencoded",
      "content-length": String(Buffer.byteLength(body)),
      ...options.headers,
    },
    body,
  });
}

function projection({
  state = "available",
  version = state === "unavailable" ? null : "1.2.3",
  sessions = [],
  error,
} = {}) {
  return {
    schemaVersion: 1,
    provider: {
      id: "codex",
      displayName: "Codex",
      state,
      version,
      capabilities: {
        recentObservation: state === "available",
        explicitLiveStatus: state === "available",
      },
      ...(error ? { error } : {}),
    },
    observedAt: "2026-07-15T12:00:00.000Z",
    sessions,
  };
}

async function markerText(marker) {
  try {
    return await readFile(marker, "utf8");
  } catch (error) {
    if (error.code === "ENOENT") return "";
    throw error;
  }
}

test("local shell enforces the browser boundary and renders durable observation states", { timeout: 90000 }, async () => {
  const { scripts } = JSON.parse(await readFile("package.json", "utf8"));
  assert.equal(scripts.dev, "node scripts/local-next.mjs dev");
  assert.equal(scripts.start, "node scripts/local-next.mjs start");
  const launcher = await readFile("scripts/local-next.mjs", "utf8");
  assert.match(launcher, /"--hostname",\s*"127\.0\.0\.1"/);
  assert.match(launcher, /PATCHFLEET_SYNC_TOKEN/);
  assert.match(launcher, /5_000/);

  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-web-data-"));
  const { binDir, geminiMode, marker } = await fakeProviders();
  let server = await startNext(dataDir, binDir);

  try {
    const initial = await fetch(server.port);
    assert.equal(initial.statusCode, 200);
    assert.match(initial.body, /Run your first controlled task/);
    assert.match(initial.body, /Three steps to controlled work/);
    assert.match(initial.body, /Local data stays local/);
    assert.match(initial.body, /No registered projects/);
    assert.match(initial.body, /patchfleet workspace add/);
    assert.match(initial.body, /Providers have not been observed/);
    assert.match(initial.body, /method="post"/);
    assert.match(initial.body, /action="\/api\/observe"/);
    assert.match(initial.body, /Patchfleet Cloud/);
    assert.match(initial.body, /action="\/api\/cloud"/);

    const internal = { path: "/api/cloud/sync", method: "POST" };
    assert.equal((await fetch(server.port, internal)).statusCode, 403, "missing sync token");
    assert.equal((await fetch(server.port, {
      ...internal,
      headers: {
        authorization: `Bearer ${server.syncToken}`,
        origin: `http://127.0.0.1:${server.port}`,
      },
    })).statusCode, 403, "browser cannot trigger internal sync");
    const unpairedSync = await fetch(server.port, {
      ...internal,
      headers: { authorization: `Bearer ${server.syncToken}` },
    });
    assert.equal(unpairedSync.statusCode, 200);
    assert.deepEqual(JSON.parse(unpairedSync.body), { kind: "unpaired" });

    for (const host of ["localhost", `localhost:${server.port}`, "127.0.0.1", `127.0.0.1:${server.port}`]) {
      const response = await fetch(server.port, { host });
      assert.equal(response.statusCode, 200, host);
      for (const [name, value] of Object.entries(requiredHeaders)) {
        assert.equal(response.headers[name], value, name);
      }
    }
    for (const host of ["example.com", "localhost.example.com", "localhost:abc", "127.0.0.1:65536"]) {
      assert.equal((await fetch(server.port, { host })).statusCode, 403, host);
    }
    assert.ok((await fetch(server.port, { host: null, setHost: false })).statusCode >= 400, "missing Host");

    const endpoint = { path: "/api/observe", method: "POST" };
    assert.equal((await fetch(server.port, endpoint)).statusCode, 403, "missing Origin");
    assert.equal((await fetch(server.port, {
      ...endpoint,
      headers: { origin: "http://example.com" },
    })).statusCode, 403, "cross Origin");
    assert.equal((await fetch(server.port, {
      ...endpoint,
      headers: { origin: `http://127.0.0.1:${server.port}`, "content-length": "1" },
      body: "x",
    })).statusCode, 403, "request body");
    assert.equal(await markerText(marker), "");
    assert.equal(await markerText(join(dataDir, "events.jsonl")), "");

    const accepted = await fetch(server.port, {
      ...endpoint,
      headers: { origin: `http://127.0.0.1:${server.port}` },
    });
    assert.equal(accepted.statusCode, 303);
    assert.equal(new URL(accepted.headers.location).origin, `http://127.0.0.1:${server.port}`);
    assert.equal(new URL(accepted.headers.location).pathname, "/");
    for (const [name, value] of Object.entries(requiredHeaders)) {
      assert.equal(accepted.headers[name], value, name);
    }
    assert.equal(await markerText(marker), "start\nstop\n");
    const stored = `${await readFile(join(dataDir, "events.jsonl"), "utf8")}${await readFile(join(dataDir, "observation.json"), "utf8")}`;
    for (const canary of [
      "CANARY_PROMPT",
      "CANARY_TRANSCRIPT",
      "/private/CANARY_PATH",
      "CLAUDE_CANARY_PROMPT",
      "/private/CLAUDE_CANARY_PATH",
    ]) {
      assert.equal(stored.includes(canary), false, canary);
    }

    const populated = await fetch(server.port);
    assert.match(populated.body, /web-sess…n-id/);
    assert.match(populated.body, /Claude Code/);
    assert.match(populated.body, /job:clau…n-id/);
    assert.match(populated.body, /Gemini CLI/);
    assert.match(populated.body, /Gemini CLI hook setup is required/);
    assert.match(populated.body, />running</);
    assert.match(populated.body, /remains available after restart/);

    const { persistGeminiLifecycleSignal, persistObservation } = await import(
      "../lib/runtime/observation-store.mjs"
    );
    await persistObservation({
      schemaVersion: 1,
      provider: {
        id: "gemini",
        displayName: "Gemini CLI",
        state: "available",
        version: "0.43.0",
        capabilities: { recentObservation: true, explicitLiveStatus: true },
      },
      observedAt: "2026-07-16T12:00:00.000Z",
      sessions: [],
    }, { dataDir });
    await persistGeminiLifecycleSignal({
      schemaVersion: 1,
      providerId: "gemini",
      providerSessionId: "gemini-review-session",
      status: "running",
      observedAt: "2026-07-16T12:01:00.000Z",
    }, { dataDir });

    await writeFile(geminiMode, "failed", "utf8");
    assert.equal((await fetch(server.port, {
      ...endpoint,
      headers: { origin: `http://127.0.0.1:${server.port}` },
    })).statusCode, 303);
    const transient = await fetch(server.port);
    assert.match(transient.body, /Gemini CLI could not be checked/);
    assert.match(transient.body, /gemini-r…sion/);

    await writeFile(geminiMode, "missing", "utf8");
    assert.equal((await fetch(server.port, {
      ...endpoint,
      headers: { origin: `http://127.0.0.1:${server.port}` },
    })).statusCode, 303);
    const setupMissing = await fetch(server.port);
    assert.match(setupMissing.body, /Gemini CLI hook setup is required/);
    assert.doesNotMatch(setupMissing.body, /gemini-r…sion/);

    const cleanLog = await readFile(join(dataDir, "events.jsonl"), "utf8");
    await writeFile(join(dataDir, "events.jsonl"), `not-json\n${cleanLog}`, "utf8");
    assert.match((await fetch(server.port)).body, /Local storage needs attention/);
    await writeFile(join(dataDir, "events.jsonl"), cleanLog, "utf8");

    await writeFile(join(dataDir, "observation.json"), `${JSON.stringify(projection())}\n`, "utf8");
    assert.match((await fetch(server.port)).body, /No recent interactive sessions were returned/);

    await writeFile(join(dataDir, "observation.json"), `${JSON.stringify(projection({
      state: "unavailable",
      error: { code: "CODEX_NOT_FOUND", message: "Codex CLI is not installed or is not on PATH." },
    }))}\n`, "utf8");
    assert.match((await fetch(server.port)).body, /Codex CLI is not installed/);

    await writeFile(join(dataDir, "observation.json"), `${JSON.stringify(projection({
      state: "degraded",
      error: { code: "CODEX_APP_SERVER_TIMEOUT", message: "Codex observation timed out." },
    }))}\n`, "utf8");
    assert.match((await fetch(server.port)).body, /Codex observation timed out/);

    await writeFile(join(dataDir, "observation.json"), "broken", "utf8");
    assert.match((await fetch(server.port)).body, /Local storage needs attention/);

    const durable = projection({
      sessions: [{
        providerSessionId: "restart-session",
        status: "unknown",
        createdAt: "2026-07-15T11:00:00.000Z",
        lastObservedAt: "2026-07-15T12:00:00.000Z",
      }],
    });
    await writeFile(join(dataDir, "observation.json"), `${JSON.stringify(durable)}\n`, "utf8");
    const markerBeforeRestart = await markerText(marker);
    await stopNext(server.child);
    server = await startNext(dataDir, binDir);
    const recovered = await fetch(server.port);
    assert.match(recovered.body, /restart-session/);
    assert.match(recovered.body, /Live status not observed/);
    assert.equal(await markerText(marker), markerBeforeRestart, "restart must not run Codex");
  } finally {
    await stopNext(server.child);
  }
});

test("local work route is bounded, capability-aware, idempotent, and restart-safe", { timeout: 90000 }, async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-web-work-data-"));
  const workspace = await mkdtemp(join(tmpdir(), "patchfleet-web-workspace-"));
  await mkdir(join(workspace, ".git"));
  const workspaceId = "workspace:99999999-9999-4999-8999-999999999999";
  const { registerWorkspace } = await import("../lib/runtime/workspace-registry.mjs");
  await registerWorkspace(workspace, {
    dataDir,
    workspaceId,
    commandId: "cmd:99999999-9999-4999-8999-999999999999",
  });
  const { binDir, controlState } = await fakeProviders();
  const createdAt = new Date();
  const commandTimes = {
    createdAt: createdAt.toISOString(),
    expiresAt: new Date(createdAt.valueOf() + 5 * 60_000).toISOString(),
  };
  const enqueueFields = {
    action: "enqueue",
    commandId: "cmd:11111111-1111-4111-8111-111111111111",
    ...commandTimes,
    title: " Route-safe work ",
    instruction: "\nPerform one safe local task.\n",
    workspaceId,
    workingDirectory: "",
  };
  const workItemId = "work:11111111-1111-4111-8111-111111111111";
  let server = await startNext(dataDir, binDir);

  try {
    const initial = await fetch(server.port);
    assert.match(initial.body, /Give Codex work/);
    assert.match(initial.body, /Choose a project/);
    assert.match(initial.body, new RegExp(`value="${workspaceId}"`));
    assert.match(initial.body, /Use another Git worktree once/);
    assert.match(initial.body, /relative paths and <code>~<\/code> are not accepted/);
    assert.match(initial.body, /Review the queued task in Your work/);
    assert.match(initial.body, /control unavailable/);
    assert.doesNotMatch(initial.body, /Start Codex/);
    for (const code of ["UNTRUSTED_CANARY", "constructor"]) {
      const unknownFeedback = await fetch(server.port, { path: `/?work=${code}` });
      assert.doesNotMatch(unknownFeedback.body, /data-testid="work-feedback"/);
    }

    const validBody = formBody(enqueueFields);
    assert.equal((await fetch(server.port, {
      path: "/api/work",
      method: "POST",
      headers: {
        "content-type": "application/x-www-form-urlencoded",
        "content-length": String(Buffer.byteLength(validBody)),
      },
      body: validBody,
    })).statusCode, 403, "missing Origin");
    assert.equal((await postForm(server.port, validBody.replace("action=enqueue", "action=enqueue&action=enqueue"))).statusCode, 403, "duplicate field");
    assert.equal((await postForm(server.port, `${validBody}&extra=true`)).statusCode, 403, "extra field");
    assert.equal((await postForm(server.port, { ...enqueueFields, instruction: "x".repeat(66_000) })).statusCode, 403, "oversized body");
    const relativePath = await postForm(server.port, {
      ...enqueueFields,
      commandId: "cmd:66666666-6666-4666-8666-666666666666",
      workspaceId: "",
      workingDirectory: "~/Patchfleet",
    });
    assert.equal(relativePath.statusCode, 303);
    const relativeTarget = new URL(relativePath.headers.location);
    assert.equal(relativeTarget.pathname, "/");
    assert.equal(relativeTarget.searchParams.get("work"), "WORKSPACE_PATH_NOT_ABSOLUTE");
    const relativeFeedback = await fetch(server.port, { path: `${relativeTarget.pathname}${relativeTarget.search}` });
    assert.match(relativeFeedback.body, /Run pwd inside the Git repository/);
    assert.match(relativeFeedback.body, /WORKSPACE_PATH_NOT_ABSOLUTE/);

    const missingWorkspace = await postForm(server.port, {
      ...enqueueFields,
      commandId: "cmd:10101010-1010-4010-8010-101010101010",
      workspaceId: "",
      workingDirectory: "",
    });
    assert.equal(new URL(missingWorkspace.headers.location).searchParams.get("work"), "WORKSPACE_SELECTION_REQUIRED");

    const conflictingWorkspace = await postForm(server.port, {
      ...enqueueFields,
      commandId: "cmd:12121212-1212-4212-8212-121212121212",
      workingDirectory: workspace,
    });
    assert.equal(new URL(conflictingWorkspace.headers.location).searchParams.get("work"), "WORKSPACE_SELECTION_CONFLICT");

    const unknownWorkspace = await postForm(server.port, {
      ...enqueueFields,
      commandId: "cmd:13131313-1313-4313-8313-131313131313",
      workspaceId: "workspace:14141414-1414-4414-8414-141414141414",
    });
    assert.equal(new URL(unknownWorkspace.headers.location).searchParams.get("work"), "WORKSPACE_NOT_REGISTERED");

    const missingTitle = await postForm(server.port, {
      ...enqueueFields,
      commandId: "cmd:77777777-7777-4777-8777-777777777777",
      title: "   ",
    });
    assert.equal(missingTitle.statusCode, 303);
    assert.equal(new URL(missingTitle.headers.location).searchParams.get("work"), "WORK_TITLE_REQUIRED");

    const enqueued = await postForm(server.port, enqueueFields);
    assert.equal(enqueued.statusCode, 303);
    const enqueuedTarget = new URL(enqueued.headers.location);
    assert.equal(enqueuedTarget.pathname, "/");
    assert.equal(enqueuedTarget.searchParams.get("work"), "WORK_ENQUEUED");
    const enqueuedFeedback = await fetch(server.port, { path: `${enqueuedTarget.pathname}${enqueuedTarget.search}` });
    assert.match(enqueuedFeedback.body, /Task added to the queue/);
    assert.match(enqueuedFeedback.body, /Local work, under control/);
    assert.doesNotMatch(enqueuedFeedback.body, /Three steps to controlled work/);
    assert.ok(enqueuedFeedback.body.indexOf("Your work") < enqueuedFeedback.body.indexOf("Give Codex work"));
    let page = await fetch(server.port);
    assert.match(page.body, /Route-safe work/);
    assert.match(page.body, /Project <strong>/);
    assert.match(page.body, /Local path/);
    assert.match(page.body, new RegExp(workspace.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    assert.doesNotMatch(page.body, /Start Codex/);

    const unavailable = await postForm(server.port, {
      action: "start",
      commandId: "cmd:44444444-4444-4444-8444-444444444444",
      ...commandTimes,
      workItemId,
      expectedItemRevision: "1",
    });
    assert.equal(unavailable.statusCode, 303);
    assert.match((await fetch(server.port)).body, /PROVIDER_CONTROL_UNAVAILABLE/);

    await stopNext(server.child);
    server = await startNext(dataDir, binDir);
    page = await fetch(server.port);
    assert.match(page.body, /Route-safe work/);
    assert.match(page.body, /PROVIDER_CONTROL_UNAVAILABLE/);

    assert.equal((await fetch(server.port, {
      path: "/api/observe",
      method: "POST",
      headers: { origin: `http://127.0.0.1:${server.port}` },
    })).statusCode, 303);
    page = await fetch(server.port);
    assert.match(page.body, /Start Codex/);

    const { persistObservation } = await import("../lib/runtime/observation-store.mjs");
    await persistObservation(projection({ version: "0.143.9" }), { dataDir });
    page = await fetch(server.port);
    assert.match(page.body, /control unavailable/);
    assert.doesNotMatch(page.body, /Start Codex/);
    const unsupported = await postForm(server.port, {
      action: "start",
      commandId: "cmd:55555555-5555-4555-8555-555555555555",
      ...commandTimes,
      workItemId,
      expectedItemRevision: "1",
    });
    assert.equal(unsupported.statusCode, 303);
    assert.match((await fetch(server.port)).body, /PROVIDER_CONTROL_UNAVAILABLE/);
    assert.equal((await fetch(server.port, {
      path: "/api/observe",
      method: "POST",
      headers: { origin: `http://127.0.0.1:${server.port}` },
    })).statusCode, 303);
    assert.match((await fetch(server.port)).body, /Start Codex/);

    const startFields = {
      action: "start",
      commandId: "cmd:22222222-2222-4222-8222-222222222222",
      ...commandTimes,
      workItemId,
      expectedItemRevision: "1",
    };
    const started = await postForm(server.port, startFields);
    assert.equal(started.statusCode, 303, started.body);
    assert.equal((await postForm(server.port, startFields)).statusCode, 303);
    page = await fetch(server.port);
    assert.match(page.body, /Cancel run/);
    assert.match(page.body, /WORK_STARTED/);
    assert.doesNotMatch(page.body, /Start Codex/);
    assert.deepEqual(
      Object.fromEntries(Object.entries(JSON.parse(await readFile(controlState, "utf8"))).filter(([key]) => key !== "threads")),
      { threadStarts: 1, turnStarts: 1, interrupts: 0 },
    );

    await stopNext(server.child);
    server = await startNext(dataDir, binDir);
    page = await fetch(server.port);
    assert.match(page.body, /Control owner changed/);
    assert.match(page.body, /WORK_STARTED/);
    assert.doesNotMatch(page.body, /Cancel run/);
    assert.equal((await fetch(server.port, {
      path: "/api/observe",
      method: "POST",
      headers: { origin: `http://127.0.0.1:${server.port}` },
    })).statusCode, 303);
    page = await fetch(server.port);
    assert.match(page.body, />blocked</);
    assert.doesNotMatch(page.body, /Cancel run/);
    assert.equal(JSON.parse(await readFile(controlState, "utf8")).interrupts, 0);
    const reconciledWork = JSON.parse(await readFile(join(dataDir, "work-items.json"), "utf8"));
    assert.deepEqual(
      [reconciledWork.items[0].status, reconciledWork.runs[0].status],
      ["blocked", "failed"],
    );

    const events = (await readFile(join(dataDir, "events.jsonl"), "utf8"))
      .trim()
      .split("\n")
      .map(JSON.parse);
    assert.equal(events.filter((item) => item.type === "run.session_lost").length, 1);
    const safeState = JSON.stringify({
      receipts: events.filter((item) => item.type === "command.receipted"),
      observation: JSON.parse(await readFile(join(dataDir, "observation.json"), "utf8")),
    });
    assert.equal(safeState.includes(enqueueFields.instruction), false);
    assert.equal(safeState.includes(workspace), false);

    const css = await readFile("app/globals.css", "utf8");
    assert.match(css, /@media \(max-width: 40rem\)[\s\S]*\.work-summary/);
    assert.match(css, /\.work-actions[\s\S]*flex-wrap: wrap/);
  } finally {
    await stopNext(server.child);
  }
});
