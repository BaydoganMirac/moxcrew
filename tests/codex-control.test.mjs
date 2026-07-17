import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { chmod, mkdir, mkdtemp, readFile, realpath, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join, parse } from "node:path";
import { test } from "node:test";
import {
  cancelCodexRun,
  closeCodexControl,
  prepareCodexWork,
  startCodexWork,
  validateCodexWorkspace,
} from "../lib/providers/codex-control.mjs";
import {
  applyWorkCommand,
  applyWorkControlCommand,
  readWorkProjection,
  reconcileWorkControlOwnership,
} from "../lib/runtime/work-queue.mjs";
import {
  commitEventTransaction,
  persistObservation,
} from "../lib/runtime/observation-store.mjs";

const FIRST = "2026-07-16T12:00:00.000Z";
const SECOND = "2026-07-16T12:01:00.000Z";
const THIRD = "2026-07-16T12:02:00.000Z";
const EXPIRES = "2026-07-16T13:00:00.000Z";
const INSTRUCTION_CANARY = "CANARY_PRIVATE_INSTRUCTION";
const NATIVE_CANARY = "CANARY_NATIVE_OUTPUT_PATH";

async function workspace() {
  const directory = await mkdtemp(join(tmpdir(), "patchfleet-control-workspace-"));
  await mkdir(join(directory, ".git"));
  return directory;
}

async function fixture({ mode = "normal" } = {}) {
  const root = await mkdtemp(join(tmpdir(), "patchfleet-control-"));
  const command = join(root, "codex");
  const stateFile = join(root, "state.json");
  const source = `#!/usr/bin/env node
const fs = require("node:fs");
const readline = require("node:readline");
const stateFile = ${JSON.stringify(stateFile)};
const mode = ${JSON.stringify(mode)};
const read = () => fs.existsSync(stateFile)
  ? JSON.parse(fs.readFileSync(stateFile, "utf8"))
  : { processes: 0, threadStarts: 0, turnStarts: 0, interrupts: 0, threads: [], serverResponse: null, timedOut: false, initialize: null, startParams: null, turnStartParams: null, interruptParams: null };
const write = (state) => fs.writeFileSync(stateFile, JSON.stringify(state));
const state = read(); state.processes += 1; write(state);
process.on("SIGTERM", () => {
  const current = read(); current.stops = (current.stops || 0) + 1; write(current); process.exit(0);
});
const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
const thread = (params, id) => ({
  id, cwd: params.cwd, threadSource: params.threadSource, ephemeral: false,
  turns: [], preview: ${JSON.stringify(NATIVE_CANARY)}, items: [{ text: ${JSON.stringify(NATIVE_CANARY)} }]
});
readline.createInterface({ input: process.stdin }).on("line", (line) => {
  const message = JSON.parse(line);
  const current = read();
  if (!message.method && message.id === "server-request-1") {
    current.serverResponse = message; write(current); return;
  }
  if (message.method === "initialized") return;
  if (message.method === "initialize") {
    current.initialize = message.params; write(current);
    return send({ id: message.id, result: { raw: ${JSON.stringify(NATIVE_CANARY)} } });
  }
  if (message.method === "thread/start") {
    current.threadStarts += 1;
    current.startParams = message.params;
    const value = thread(message.params, "thread-" + current.threadStarts);
    current.threads.push(value);
    const timeout = mode === "timeout-thread-once" && !current.timedOut;
    current.timedOut ||= timeout; write(current);
    if (timeout) return;
    send({ id: message.id, result: {
      thread: mode === "malformed-thread" ? { ...value, cwd: "/unsafe" } : value,
      raw: ${JSON.stringify(NATIVE_CANARY)}
    } });
    return send({ id: "server-request-1", method: "item/tool/requestUserInput", params: { raw: ${JSON.stringify(NATIVE_CANARY)} } });
  }
  if (message.method === "turn/start") {
    const value = current.threads.find((item) => item.id === message.params.threadId);
    const turn = { id: "turn-1", status: "inProgress", completedAt: null, items: [{ text: ${JSON.stringify(NATIVE_CANARY)} }] };
    value.turns = [turn]; current.turnStarts += 1; current.turnStartParams = message.params;
    const timeout = mode === "timeout-turn-once" && !current.timedOut;
    current.timedOut ||= timeout; write(current);
    if (!timeout) send({ id: message.id, result: {
      turn: mode === "malformed-turn" ? { ...turn, id: "unsafe id" } : turn,
      raw: ${JSON.stringify(NATIVE_CANARY)}
    } });
    return;
  }
  if (message.method === "turn/interrupt") {
    const value = current.threads.find((item) => item.id === message.params.threadId);
    value.turns[0].status = "interrupted"; value.turns[0].completedAt = 1784200000;
    current.interrupts += 1; current.interruptParams = message.params;
    const timeout = mode === "timeout-interrupt-once" && !current.timedOut;
    current.timedOut ||= timeout; write(current);
    if (!timeout) return send({ id: message.id, result: mode === "malformed-interrupt" ? null : {} });
    return;
  }
});
`;
  await writeFile(command, source, "utf8");
  await chmod(command, 0o700);
  return {
    command,
    state: async () => {
      try {
        return JSON.parse(await readFile(stateFile, "utf8"));
      } catch (error) {
        if (error.code === "ENOENT") return { processes: 0, threadStarts: 0, turnStarts: 0 };
        throw error;
      }
    },
  };
}

function enqueue(cwd) {
  return {
    schemaVersion: 1,
    intentId: "queue-work",
    idempotencyKey: "queue-work",
    type: "enqueue_work",
    actorId: "local-owner",
    createdAt: FIRST,
    expiresAt: EXPIRES,
    payload: { workItem: {
      schemaVersion: 1,
      workItemId: "work-1",
      title: "Safe work",
      instruction: INSTRUCTION_CANARY,
      providerId: "codex",
      workingDirectory: cwd,
      status: "queued",
      createdAt: FIRST,
      revision: 1,
    } },
  };
}

function start(intentId = "control-start") {
  return {
    schemaVersion: 1,
    intentId,
    idempotencyKey: intentId,
    type: "start_work",
    actorId: "local-owner",
    createdAt: SECOND,
    expiresAt: EXPIRES,
    payload: { workItemId: "work-1", expectedItemRevision: 1 },
  };
}

function cancel() {
  return {
    schemaVersion: 1,
    intentId: "control-cancel",
    idempotencyKey: "control-cancel",
    type: "cancel_run",
    actorId: "local-owner",
    createdAt: THIRD,
    expiresAt: EXPIRES,
    payload: { runId: "run:control-start", expectedRunRevision: 1 },
  };
}

function workEvent(type, recordedAt, payload) {
  return { id: randomUUID(), schemaVersion: 1, type, recordedAt, payload };
}

function appendWorkFacts(dataDir, additions) {
  return commitEventTransaction(() => ({ additions, result: () => undefined }), { dataDir });
}

function launchingFacts(ownerEpoch) {
  const intent = start();
  return [
    workEvent("command.requested", SECOND, { intent }),
    workEvent("run.launching", SECOND, {
      intentId: intent.intentId,
      workItemId: intent.payload.workItemId,
      expectedItemRevision: intent.payload.expectedItemRevision,
      ownerEpoch,
    }),
  ];
}

function terminalObservation(status) {
  return {
    provider: {
      id: "codex",
      displayName: "Codex",
      state: "available",
      version: "1.2.3",
      capabilities: { recentObservation: true, explicitLiveStatus: true },
    },
    observedAt: THIRD,
    sessions: [{
      providerSessionId: "thread-1",
      status,
      createdAt: FIRST,
      lastObservedAt: THIRD,
      terminalAt: THIRD,
    }],
  };
}

test("start uses one bounded app-server, rejects server requests, and persists only safe run ids", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const canonicalCwd = await realpath(cwd);
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    const applied = await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    assert.deepEqual([applied.outcome, applied.reasonCode], ["applied", "WORK_STARTED"]);
    assert.deepEqual(
      await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
      }),
      applied,
    );
    await new Promise((resolve) => setTimeout(resolve, 20));
    const native = await fake.state();
    assert.deepEqual([native.processes, native.threadStarts, native.turnStarts], [1, 1, 1]);
    assert.deepEqual(native.initialize, {
      clientInfo: { name: "patchfleet", title: "Patchfleet", version: "0.1.0" },
      capabilities: null,
    });
    assert.deepEqual(native.startParams, {
      cwd: canonicalCwd,
      approvalPolicy: "never",
      sandbox: "workspace-write",
      ephemeral: false,
      threadSource: "patchfleet:control-start",
      experimentalRawEvents: false,
    });
    assert.deepEqual(native.turnStartParams, {
      threadId: "thread-1",
      clientUserMessageId: "control-start",
      input: [{ type: "text", text: INSTRUCTION_CANARY, text_elements: [] }],
    });
    assert.deepEqual(native.serverResponse, {
      id: "server-request-1",
      error: { code: -32601, message: "Method not found" },
    });
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual(projection.runs.map((run) => [run.runId, run.providerSessionId, run.providerTurnId, run.status]), [
      ["run:control-start", "thread-1", "turn-1", "running"],
    ]);
    assert.equal(JSON.stringify(projection.receipts).includes(INSTRUCTION_CANARY), false);
    assert.equal(JSON.stringify(projection).includes(NATIVE_CANARY), false);
    const log = await readFile(join(dataDir, "events.jsonl"), "utf8");
    assert.equal(log.includes(NATIVE_CANARY), false);
    assert.equal(log.includes('"raw"'), false);

    await persistObservation({
      provider: {
        id: "codex",
        displayName: "Codex",
        state: "available",
        version: "1.2.3",
        capabilities: { recentObservation: true, explicitLiveStatus: true },
      },
      observedAt: THIRD,
      sessions: [{
        providerSessionId: "thread-1",
        status: "completed",
        createdAt: FIRST,
        lastObservedAt: THIRD,
        terminalAt: THIRD,
      }],
    }, { dataDir });
    const reconciled = await readWorkProjection({ dataDir });
    assert.deepEqual([reconciled.revision, reconciled.items[0].status, reconciled.runs[0].status], [3, "completed", "completed"]);
  } finally {
    await closeCodexControl();
  }
});

test("an uncertain turn start is terminal in the same call and exact retry does not duplicate work", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture({ mode: "timeout-turn-once" });
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    const failed = await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
    assert.deepEqual(
      await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
      }),
      failed,
    );
    const native = await fake.state();
    assert.deepEqual([native.threadStarts, native.turnStarts], [1, 1]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual([projection.items[0].status, projection.runs.length], ["blocked", 0]);
  } finally {
    await closeCodexControl();
  }
});

test("an uncertain thread start is terminal in the same call and same-owner retry starts no replacement", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture({ mode: "timeout-thread-once" });
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    const failed = await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
    assert.deepEqual(
      await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
      }),
      failed,
    );
    const native = await fake.state();
    assert.deepEqual([native.threadStarts, native.turnStarts], [1, 0]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual([projection.items[0].status, projection.runs.length], ["blocked", 0]);
  } finally {
    await closeCodexControl();
  }
});

test("an owner crash before provider prepare blocks the launch without starting a provider thread", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await appendWorkFacts(dataDir, launchingFacts("owner-before-prepare"));
    assert.equal((await readWorkProjection({ dataDir })).items[0].status, "launching");

    const failed = await applyWorkControlCommand(start(), {
      dataDir,
      command: fake.command,
      now: () => THIRD,
      ownerEpoch: "owner-after-restart",
      timeoutMs: 500,
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
    assert.deepEqual(await fake.state(), { processes: 0, threadStarts: 0, turnStarts: 0 });
    assert.equal((await readWorkProjection({ dataDir })).items[0].status, "blocked");
  } finally {
    await closeCodexControl();
  }
});

test("an owner crash after provider prepare leaves one empty thread and never starts a replacement", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await appendWorkFacts(dataDir, launchingFacts("owner-before-prepare-checkpoint"));
    await prepareCodexWork({
      intentId: start().intentId,
      workingDirectory: cwd,
      command: fake.command,
      timeoutMs: 500,
    });
    await closeCodexControl();
    assert.equal(
      (await readFile(join(dataDir, "events.jsonl"), "utf8")).includes('"type":"run.prepared"'),
      false,
    );

    const failed = await applyWorkControlCommand(start(), {
      dataDir,
      command: fake.command,
      now: () => THIRD,
      ownerEpoch: "owner-after-prepare-crash",
      timeoutMs: 500,
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
    const native = await fake.state();
    assert.deepEqual([native.threadStarts, native.turnStarts], [1, 0]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual([projection.items[0].status, projection.runs.length], ["blocked", 0]);
  } finally {
    await closeCodexControl();
  }
});

test("an owner crash after turn.requested blocks the launch without a turn or replacement thread", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    const ownerEpoch = "owner-before-turn";
    await appendWorkFacts(dataDir, launchingFacts(ownerEpoch));
    const prepared = await prepareCodexWork({
      intentId: start().intentId,
      workingDirectory: cwd,
      command: fake.command,
      timeoutMs: 500,
    });
    await appendWorkFacts(dataDir, [
      workEvent("run.prepared", SECOND, {
        intentId: start().intentId,
        workItemId: start().payload.workItemId,
        providerId: "codex",
        providerSessionId: prepared.providerSessionId,
        expectedItemRevision: start().payload.expectedItemRevision,
        ownerEpoch,
      }),
      workEvent("turn.requested", SECOND, {
        intentId: start().intentId,
        providerSessionId: prepared.providerSessionId,
        ownerEpoch,
      }),
    ]);
    await closeCodexControl();
    const preparedLog = await readFile(join(dataDir, "events.jsonl"), "utf8");
    assert.equal(preparedLog.includes('"type":"run.prepared"'), true);
    assert.equal(preparedLog.includes(NATIVE_CANARY), false);

    const failed = await applyWorkControlCommand(start(), {
      dataDir,
      command: fake.command,
      now: () => THIRD,
      ownerEpoch: "owner-after-turn-crash",
      timeoutMs: 500,
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
    const native = await fake.state();
    assert.deepEqual([native.threadStarts, native.turnStarts], [1, 0]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual([projection.items[0].status, projection.runs.length], ["blocked", 0]);
    assert.equal(JSON.stringify(projection).includes(NATIVE_CANARY), false);
  } finally {
    await closeCodexControl();
  }
});

test("same-owner cancel interrupts once and duplicate delivery returns the receipt", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    const applied = await applyWorkControlCommand(cancel(), {
      dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
    });
    assert.deepEqual([applied.outcome, applied.reasonCode], ["applied", "RUN_CANCELLED"]);
    assert.deepEqual(
      await applyWorkControlCommand(cancel(), {
        dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
      }),
      applied,
    );
    const native = await fake.state();
    assert.equal(native.interrupts, 1);
    assert.deepEqual(native.interruptParams, { threadId: "thread-1", turnId: "turn-1" });
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual([projection.items[0].status, projection.runs[0].status], ["interrupted", "interrupted"]);
  } finally {
    await closeCodexControl();
  }
});

test("a concurrent interrupted observation preserves one applied cancel fact without another revision", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    const applied = await applyWorkControlCommand(cancel(), {
      dataDir,
      command: fake.command,
      now: () => THIRD,
      timeoutMs: 500,
      cancel: async (options) => {
        await cancelCodexRun(options);
        await persistObservation(terminalObservation("interrupted"), { dataDir });
      },
    });
    assert.deepEqual([applied.outcome, applied.reasonCode], ["applied", "RUN_CANCELLED"]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual(
      [projection.revision, applied.workProjectionRevision, projection.items[0].status, projection.runs[0].status],
      [3, 3, "interrupted", "interrupted"],
    );
    const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
    assert.equal(events.filter((item) =>
      item.type === "run.interrupted" && item.payload.intentId === cancel().intentId).length, 1);
    assert.equal((await fake.state()).interrupts, 1);
  } finally {
    await closeCodexControl();
  }
});

test("a concurrent completed or failed observation does not claim the cancel applied", async (t) => {
  for (const status of ["completed", "failed"]) {
    await t.test(status, async () => {
      const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
      const cwd = await workspace();
      const fake = await fixture();
      try {
        await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
        await applyWorkControlCommand(start(), {
          dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
        });
        const failed = await applyWorkControlCommand(cancel(), {
          dataDir,
          command: fake.command,
          now: () => THIRD,
          timeoutMs: 500,
          cancel: () => persistObservation(terminalObservation(status), { dataDir }),
        });
        assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "PROVIDER_CONTROL_FAILED"]);
        const projection = await readWorkProjection({ dataDir });
        assert.deepEqual(
          [projection.revision, failed.workProjectionRevision, projection.runs[0].status],
          [3, 3, status],
        );
        const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
        assert.equal(events.some((item) =>
          item.type === "run.interrupted" && item.payload.intentId === cancel().intentId), false);
        assert.equal((await fake.state()).interrupts, 0);
      } finally {
        await closeCodexControl();
      }
    });
  }
});

test("an uncertain cancel preserves RUN_SESSION_LOST when observation already failed the run", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    const failed = await applyWorkControlCommand(cancel(), {
      dataDir,
      command: fake.command,
      now: () => THIRD,
      timeoutMs: 500,
      cancel: async () => {
        await persistObservation(terminalObservation("failed"), { dataDir });
        throw Object.assign(new Error("connection lost"), { outcomeUnknown: true });
      },
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "RUN_SESSION_LOST"]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual([projection.revision, projection.runs[0].status], [3, "failed"]);
    const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
    assert.equal(events.some((item) => item.type === "run.session_lost"), false);
  } finally {
    await closeCodexControl();
  }
});

test("exact retry recovers terminal receipts from durable control facts", async (t) => {
  await t.test("run.started", async () => {
    const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
    const cwd = await workspace();
    const fake = await fixture();
    const ownerEpoch = "owner-started-fact";
    try {
      await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
      await appendWorkFacts(dataDir, launchingFacts(ownerEpoch));
      const prepared = await prepareCodexWork({
        intentId: start().intentId,
        workingDirectory: cwd,
        command: fake.command,
        timeoutMs: 500,
      });
      await appendWorkFacts(dataDir, [
        workEvent("run.prepared", SECOND, {
          intentId: start().intentId,
          workItemId: start().payload.workItemId,
          providerId: "codex",
          providerSessionId: prepared.providerSessionId,
          expectedItemRevision: start().payload.expectedItemRevision,
          ownerEpoch,
        }),
        workEvent("turn.requested", SECOND, {
          intentId: start().intentId,
          providerSessionId: prepared.providerSessionId,
          ownerEpoch,
        }),
      ]);
      const started = await startCodexWork({
        intentId: start().intentId,
        instruction: INSTRUCTION_CANARY,
        providerSessionId: prepared.providerSessionId,
        workingDirectory: cwd,
        command: fake.command,
        timeoutMs: 500,
      });
      await appendWorkFacts(dataDir, [workEvent("run.started", SECOND, {
        intentId: start().intentId,
        expectedItemRevision: start().payload.expectedItemRevision,
        ownerEpoch,
        run: {
          schemaVersion: 1,
          runId: `run:${start().intentId}`,
          workItemId: start().payload.workItemId,
          providerId: "codex",
          ownerEpoch,
          providerSessionId: started.providerSessionId,
          providerTurnId: started.providerTurnId,
          status: started.status,
          startedAt: SECOND,
          terminalAt: started.terminalAt,
          revision: 1,
        },
      })]);
      const recovered = await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => THIRD, ownerEpoch, timeoutMs: 500,
      });
      assert.deepEqual([recovered.outcome, recovered.reasonCode], ["applied", "WORK_STARTED"]);
      assert.deepEqual(
        [(await fake.state()).threadStarts, (await fake.state()).turnStarts],
        [1, 1],
      );
    } finally {
      await closeCodexControl();
    }
  });

  await t.test("run.start_unknown", async () => {
    const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
    const cwd = await workspace();
    const fake = await fixture();
    const ownerEpoch = "owner-unknown-fact";
    try {
      await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
      await appendWorkFacts(dataDir, [
        ...launchingFacts(ownerEpoch),
        workEvent("run.start_unknown", SECOND, {
          intentId: start().intentId,
          workItemId: start().payload.workItemId,
          expectedItemRevision: start().payload.expectedItemRevision,
          ownerEpoch,
        }),
      ]);
      const recovered = await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => THIRD, ownerEpoch, timeoutMs: 500,
      });
      assert.deepEqual([recovered.outcome, recovered.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
      assert.deepEqual(await fake.state(), { processes: 0, threadStarts: 0, turnStarts: 0 });
    } finally {
      await closeCodexControl();
    }
  });

  await t.test("run.interrupted after a receipt crash tail", async () => {
    const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
    const cwd = await workspace();
    const fake = await fixture();
    try {
      await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
      await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
      });
      const running = await readWorkProjection({ dataDir });
      const run = running.runs[0];
      await cancelCodexRun({
        runId: run.runId,
        providerSessionId: run.providerSessionId,
        providerTurnId: run.providerTurnId,
        workingDirectory: cwd,
        command: fake.command,
        timeoutMs: 500,
      });
      await appendWorkFacts(dataDir, [
        workEvent("command.requested", THIRD, { intent: cancel() }),
        workEvent("run.interrupted", THIRD, {
          intentId: cancel().intentId,
          runId: run.runId,
          expectedRunRevision: run.revision,
        }),
      ]);
      const logPath = join(dataDir, "events.jsonl");
      await writeFile(logPath, `${await readFile(logPath, "utf8")}{"partial"`, "utf8");
      const recovered = await applyWorkControlCommand(cancel(), {
        dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
      });
      assert.deepEqual([recovered.outcome, recovered.reasonCode], ["applied", "RUN_CANCELLED"]);
      assert.equal((await fake.state()).interrupts, 1);
      const events = (await readFile(logPath, "utf8")).trim().split("\n").map(JSON.parse);
      assert.equal(events.filter((item) => item.type === "run.interrupted").length, 1);
      assert.equal(events.filter((item) =>
        item.type === "command.receipted" && item.payload.receipt.intentId === cancel().intentId).length, 1);
    } finally {
      await closeCodexControl();
    }
  });

  await t.test("run.session_lost", async () => {
    const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
    const cwd = await workspace();
    const fake = await fixture();
    const ownerEpoch = "owner-session-fact";
    try {
      await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
      await applyWorkControlCommand(start(), {
        dataDir, command: fake.command, now: () => SECOND, ownerEpoch, timeoutMs: 500,
      });
      await appendWorkFacts(dataDir, [
        workEvent("command.requested", THIRD, { intent: cancel() }),
        workEvent("run.session_lost", THIRD, {
          runId: cancel().payload.runId,
          ownerEpoch,
        }),
      ]);
      const recovered = await applyWorkControlCommand(cancel(), {
        dataDir, command: fake.command, now: () => THIRD, ownerEpoch, timeoutMs: 500,
      });
      assert.deepEqual([recovered.outcome, recovered.reasonCode], ["failed", "RUN_SESSION_LOST"]);
      assert.equal((await fake.state()).interrupts, 0);
    } finally {
      await closeCodexControl();
    }
  });
});

test("an uncertain interrupt writes one terminal receipt and exact retry does not interrupt twice", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture({ mode: "timeout-interrupt-once" });
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await applyWorkControlCommand(start(), {
      dataDir, command: fake.command, now: () => SECOND, timeoutMs: 500,
    });
    const failed = await applyWorkControlCommand(cancel(), {
      dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
    });
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "RUN_SESSION_LOST"]);
    assert.deepEqual(
      await applyWorkControlCommand(cancel(), {
        dataDir, command: fake.command, now: () => THIRD, timeoutMs: 500,
      }),
      failed,
    );
    const native = await fake.state();
    assert.deepEqual([native.interrupts, native.stops], [1, 1]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual(
      [projection.items[0].status, projection.runs[0].status],
      ["blocked", "failed"],
    );
  } finally {
    await closeCodexControl();
  }
});

test("malformed post-side-effect responses close control and fail closed", async (t) => {
  for (const mode of ["malformed-thread", "malformed-turn", "malformed-interrupt"]) {
    await t.test(mode, async () => {
      const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
      const cwd = await workspace();
      const fake = await fixture({ mode });
      try {
        await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
        const started = await applyWorkControlCommand(start(), {
          dataDir, command: fake.command, now: () => SECOND, timeoutMs: 2_000,
        });
        if (mode === "malformed-interrupt") {
          assert.equal(started.outcome, "applied");
          const failed = await applyWorkControlCommand(cancel(), {
            dataDir, command: fake.command, now: () => THIRD, timeoutMs: 2_000,
          });
          assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "RUN_SESSION_LOST"]);
        } else {
          assert.deepEqual([started.outcome, started.reasonCode], ["failed", "START_OUTCOME_UNKNOWN"]);
        }
        const native = await fake.state();
        assert.equal(native.stops, 1);
        assert.deepEqual(
          [native.threadStarts, native.turnStarts, native.interrupts],
          mode === "malformed-thread" ? [1, 0, 0]
            : mode === "malformed-turn" ? [1, 1, 0]
              : [1, 1, 1],
        );
        const projection = await readWorkProjection({ dataDir });
        assert.deepEqual(
          [projection.items[0].status, projection.runs.length ? projection.runs[0].status : null],
          mode === "malformed-interrupt" ? ["blocked", "failed"] : ["blocked", null],
        );
      } finally {
        await closeCodexControl();
      }
    });
  }
});

test("restart reconciliation terminalizes an unreceipted old-owner cancel", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await applyWorkControlCommand(start(), {
      dataDir,
      command: fake.command,
      now: () => SECOND,
      ownerEpoch: "owner-before-cancel-crash",
      timeoutMs: 500,
    });
    await appendWorkFacts(dataDir, [workEvent("command.requested", THIRD, { intent: cancel() })]);
    assert.equal((await readWorkProjection({ dataDir })).runs[0].status, "cancelling");
    await closeCodexControl();

    await reconcileWorkControlOwnership({
      dataDir,
      now: () => THIRD,
      ownerEpoch: "owner-after-cancel-crash",
    });
    const projection = await readWorkProjection({ dataDir });
    const failed = projection.receipts.find((item) => item.intentId === cancel().intentId);
    assert.deepEqual([failed.outcome, failed.reasonCode], ["failed", "RUN_SESSION_LOST"]);
    assert.deepEqual(
      [projection.items[0].status, projection.runs[0].status],
      ["blocked", "failed"],
    );
    assert.deepEqual(
      await applyWorkControlCommand(cancel(), {
        dataDir,
        command: fake.command,
        now: () => THIRD,
        ownerEpoch: "owner-after-cancel-crash",
        timeoutMs: 500,
      }),
      failed,
    );
    assert.equal((await fake.state()).interrupts, 0);
    const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
    assert.equal(events.filter((item) =>
      item.type === "command.receipted" && item.payload.receipt.intentId === cancel().intentId).length, 1);
  } finally {
    await closeCodexControl();
  }
});

test("an active run from an old owner is session-lost and never interrupted by a replacement owner", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    await applyWorkControlCommand(start(), {
      dataDir,
      command: fake.command,
      now: () => SECOND,
      ownerEpoch: "owner-active-run",
      timeoutMs: 500,
    });

    const rejected = await applyWorkControlCommand(cancel(), {
      dataDir,
      command: fake.command,
      now: () => THIRD,
      ownerEpoch: "owner-after-active-crash",
      timeoutMs: 500,
    });
    assert.deepEqual([rejected.outcome, rejected.reasonCode], ["rejected", "RUN_NOT_ACTIVE"]);
    const native = await fake.state();
    assert.deepEqual([native.threadStarts, native.turnStarts, native.interrupts], [1, 1, 0]);
    const projection = await readWorkProjection({ dataDir });
    assert.deepEqual(
      [projection.items[0].status, projection.runs[0].status],
      ["blocked", "failed"],
    );
  } finally {
    await closeCodexControl();
  }
});

test("workspace preflight fails closed before a provider thread can start", async () => {
  await assert.rejects(() => validateCodexWorkspace(parse(homedir()).root), { code: "WORKSPACE_NOT_ALLOWED" });
  await assert.rejects(() => validateCodexWorkspace(homedir()), { code: "WORKSPACE_NOT_ALLOWED" });
  await assert.rejects(() => validateCodexWorkspace(join(tmpdir(), "patchfleet-missing")), { code: "WORKSPACE_NOT_ALLOWED" });
  const noGit = await mkdtemp(join(tmpdir(), "patchfleet-no-git-"));
  await assert.rejects(() => validateCodexWorkspace(noGit), { code: "WORKSPACE_NOT_ALLOWED" });

  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-control-data-"));
  const cwd = await workspace();
  const fake = await fixture();
  try {
    await applyWorkCommand(enqueue(cwd), { dataDir, now: () => FIRST });
    const rejected = await applyWorkControlCommand(start(), {
      dataDir,
      command: fake.command,
      now: () => SECOND,
      timeoutMs: 500,
      prepare: ({ workingDirectory }) => prepareCodexWork({
        intentId: "control-start",
        workingDirectory: join(workingDirectory, "missing"),
        command: fake.command,
        timeoutMs: 500,
      }),
    });
    assert.deepEqual([rejected.outcome, rejected.reasonCode], ["rejected", "WORKSPACE_NOT_ALLOWED"]);
    assert.equal((await fake.state()).threadStarts, 0);
  } finally {
    await closeCodexControl();
  }
});
