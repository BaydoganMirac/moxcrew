import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { projectAgentTeamEvents, validateAgentTeam } from "../lib/domain/agent-team.mjs";
import { persistObservation } from "../lib/runtime/observation-store.mjs";
import {
  advanceAgentTeam,
  answerTeamQuestion,
  cancelAgentTeam,
  cancelTeamAgent,
  createAgentTeam,
  decideTeamApproval,
  readAgentTeamProjection,
  startAgentTeam,
} from "../lib/runtime/team-orchestrator.mjs";
import { applyWorkControlCommand } from "../lib/runtime/work-queue.mjs";
import { registerWorkspace } from "../lib/runtime/workspace-registry.mjs";

const times = [
  "2026-07-20T11:00:00.000Z", "2026-07-20T11:00:01.000Z", "2026-07-20T11:00:02.000Z",
  "2026-07-20T11:00:03.000Z", "2026-07-20T11:00:04.000Z", "2026-07-20T11:00:05.000Z",
  "2026-07-20T11:00:06.000Z", "2026-07-20T11:00:07.000Z", "2026-07-20T11:00:08.000Z",
  "2026-07-20T11:00:09.000Z", "2026-07-20T11:00:10.000Z", "2026-07-20T11:00:11.000Z",
];

function clock() {
  let index = 0;
  return () => times[Math.min(index++, times.length - 1)];
}

async function setup(prefix) {
  const dataDir = await mkdtemp(join(tmpdir(), `${prefix}-data-`));
  const workspace = await mkdtemp(join(tmpdir(), `${prefix}-workspace-`));
  await mkdir(join(workspace, ".git"));
  const workspaceId = "workspace:11111111-1111-4111-8111-111111111111";
  await registerWorkspace(workspace, {
    dataDir,
    workspaceId,
    commandId: "cmd:11111111-1111-4111-8111-111111111111",
    now: () => "2026-07-20T10:59:00.000Z",
  });
  return { dataDir, workspace, workspaceId };
}

function fakeControl(status = "running") {
  return (intent, options) => applyWorkControlCommand(intent, {
    ...options,
    prepare: async ({ intentId }) => ({ providerSessionId: `session:${intentId}` }),
    start: async ({ intentId, providerSessionId }) => ({
      providerSessionId,
      providerTurnId: `turn:${intentId}`,
      status,
      terminalAt: status === "running" ? null : options.now(),
    }),
    cancel: async () => undefined,
  });
}

async function completeSessions(dataDir, sessionIds, observedAt) {
  await persistObservation({
    provider: {
      id: "codex",
      displayName: "Codex",
      state: "available",
      version: "0.144.0",
      capabilities: { recentObservation: true, explicitLiveStatus: true },
    },
    observedAt,
    sessions: sessionIds.map((providerSessionId) => ({
      providerSessionId,
      status: "completed",
      createdAt: observedAt,
      lastObservedAt: observedAt,
      terminalAt: observedAt,
    })),
  }, { dataDir });
}

test("product team validates a DAG, enforces concurrency, advances dependencies, and rebuilds", async () => {
  const { dataDir, workspaceId } = await setup("patchfleet-team-flow");
  const now = clock();
  const team = await createAgentTeam({
    name: "Customer settings",
    goal: "Deliver an accessible customer settings slice with server validation and tests.",
    workspaceId,
    templateId: "product-feature",
    workerPackIds: ["pack:product", "pack:backend", "pack:frontend", "pack:qa"],
    settings: { concurrency: 2, retryLimit: 1, timeoutMinutes: 120, failurePolicy: "stop" },
  }, { dataDir, now, teamId: "team:22222222-2222-4222-8222-222222222222" });
  assert.equal(validateAgentTeam(team).tasks.length, 4);
  assert.equal(team.tasks.filter((task) => task.dependsOn.length === 0).length, 1);

  let state = await startAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.status, "active");
  assert.deepEqual(state.tasks.map((task) => task.status), ["running", "pending", "pending", "pending"]);
  const workAfterPlan = JSON.parse(await readFile(join(dataDir, "work-items.json"), "utf8"));
  await completeSessions(dataDir, workAfterPlan.runs.map((run) => run.providerSessionId), times[5]);
  state = await advanceAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.tasks.filter((task) => task.status === "running").length, 2, "backend and frontend run in parallel");
  const workAfterImplementation = JSON.parse(await readFile(join(dataDir, "work-items.json"), "utf8"));
  await completeSessions(dataDir, workAfterImplementation.runs.filter((run) => run.status === "running").map((run) => run.providerSessionId), times[8]);
  state = await advanceAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.tasks.at(-1).status, "running");
  const workAfterQa = JSON.parse(await readFile(join(dataDir, "work-items.json"), "utf8"));
  await completeSessions(dataDir, workAfterQa.runs.filter((run) => run.status === "running").map((run) => run.providerSessionId), times[10]);
  state = await advanceAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.status, "completed");

  await writeFile(join(dataDir, "agent-teams.json"), "broken", "utf8");
  state = (await readAgentTeamProjection({ dataDir })).items[0];
  assert.equal(state.status, "completed");
  assert.equal(JSON.stringify({ team: state }).includes("customer settings slice"), true, "team goal remains canonical only in Local state");
});

test("release team requires an owner answer and approval before starting release work", async () => {
  const { dataDir, workspaceId } = await setup("patchfleet-team-gates");
  const now = clock();
  const team = await createAgentTeam({
    name: "Beta release",
    goal: "Prepare a rollback-safe beta release without deploying it.",
    workspaceId,
    templateId: "release",
    workerPackIds: ["pack:qa", "pack:release"],
  }, { dataDir, now, teamId: "team:33333333-3333-4333-8333-333333333333" });
  let state = await startAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  const work = JSON.parse(await readFile(join(dataDir, "work-items.json"), "utf8"));
  await completeSessions(dataDir, work.runs.map((run) => run.providerSessionId), times[5]);
  state = await advanceAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  const releaseTask = state.tasks.find((task) => task.approvalRequired);
  assert.equal(releaseTask.status, "waiting_question");
  await assert.rejects(() => decideTeamApproval(team.teamId, releaseTask.taskId, "approved", null, { dataDir, now }), /not available/);
  await answerTeamQuestion(team.teamId, releaseTask.question.questionId, "Staging beta; prepare only, do not deploy.", { dataDir, now });
  state = (await readAgentTeamProjection({ dataDir })).items[0];
  assert.equal(state.tasks.find((task) => task.taskId === releaseTask.taskId).status, "waiting_approval");
  await decideTeamApproval(team.teamId, releaseTask.taskId, "approved", "Run local release checks only.", { dataDir, now });
  state = await advanceAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.tasks.find((task) => task.taskId === releaseTask.taskId).status, "running");
  const latestWork = JSON.parse(await readFile(join(dataDir, "work-items.json"), "utf8"));
  const releaseWork = latestWork.items.find((item) => item.workItemId === state.tasks.find((task) => task.taskId === releaseTask.taskId).workItemId);
  assert.match(releaseWork.instruction, /Staging beta; prepare only/);
  assert.match(releaseWork.instruction, /never widen the provider sandbox/);
});

test("failed work retries once and team cancellation stops scheduling and active runs", async () => {
  const { dataDir, workspaceId } = await setup("patchfleet-team-retry");
  const now = clock();
  const team = await createAgentTeam({
    name: "Retry boundary",
    goal: "Prove one bounded failure and retry path.",
    workspaceId,
    templateId: "bug-fix",
    workerPackIds: ["pack:fullstack", "pack:qa"],
    settings: { concurrency: 1, retryLimit: 1, timeoutMinutes: 30, failurePolicy: "stop" },
  }, { dataDir, now, teamId: "team:44444444-4444-4444-8444-444444444444" });
  let state = await startAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl("failed") });
  assert.equal(state.tasks[0].canRetry, true);
  state = await advanceAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.tasks[0].attempt, 2);
  assert.equal(state.tasks[0].status, "running");
  state = await cancelAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.status, "cancelled");
  assert.equal(state.tasks.every((task) => task.status === "cancelled"), true);
  const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
  assert.equal(events.filter((event) => event.type === "team.task.retry_requested").length, 1);
  assert.equal(events.filter((event) => event.type === "team.cancelled").length, 1);
  assert.equal(events.some((event) => event.type === "run.interrupted"), true);
});

test("team DAG projection supports out-of-order task declarations and rejects cycles", async () => {
  const { dataDir, workspaceId } = await setup("patchfleet-team-dag-order");
  const team = await createAgentTeam({
    name: "DAG order",
    goal: "Prove graph meaning is independent from JSON array order.",
    workspaceId,
    templateId: "product-feature",
    workerPackIds: ["pack:product", "pack:frontend", "pack:qa"],
  }, { dataDir, now: clock(), teamId: "team:55555555-5555-4555-8555-555555555555" });
  const reversed = { ...team, tasks: [...team.tasks].reverse() };
  const events = [
    { id: "event-1", schemaVersion: 1, type: "team.created", recordedAt: times[0], payload: { team: reversed } },
    { id: "event-2", schemaVersion: 1, type: "team.started", recordedAt: times[1], payload: { teamId: team.teamId } },
  ];
  const projected = projectAgentTeamEvents(events).items[0];
  assert.equal(projected.tasks.find((task) => task.dependsOn.length === 0).status, "ready");
  assert.equal(projected.tasks.find((task) => task.dependsOn.length > 0).status, "pending");
  const cyclic = structuredClone(team);
  cyclic.tasks[0].dependsOn = [cyclic.tasks.at(-1).taskId];
  assert.throws(() => validateAgentTeam(cyclic), /cycle/);
});

test("team timeout and individual agent cancellation are durable terminal controls", async () => {
  const { dataDir, workspaceId } = await setup("patchfleet-team-terminal-controls");
  const team = await createAgentTeam({
    name: "Time boundary",
    goal: "Prove timeout and individual cancellation controls.",
    workspaceId,
    templateId: "bug-fix",
    workerPackIds: ["pack:fullstack", "pack:qa"],
    settings: { concurrency: 1, retryLimit: 0, timeoutMinutes: 5, failurePolicy: "stop" },
  }, { dataDir, now: () => "2026-07-20T10:00:00.000Z", teamId: "team:66666666-6666-4666-8666-666666666666" });
  let state = await startAgentTeam(team.teamId, { dataDir, now: () => "2026-07-20T10:00:01.000Z", providerAvailable: true, applyControl: fakeControl() });
  const activeAgent = state.agents.find((agent) => agent.agentId !== state.orchestratorAgentId && agent.status === "running");
  state = await cancelTeamAgent(team.teamId, activeAgent.agentId, { dataDir, now: () => "2026-07-20T10:00:02.000Z", providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.agents.find((agent) => agent.agentId === activeAgent.agentId).status, "cancelled");
  assert.equal(state.tasks.filter((task) => task.agentId === activeAgent.agentId).every((task) => task.status === "cancelled"), true);

  const timeoutTeam = await createAgentTeam({
    name: "Timeout only",
    goal: "Stop work after the declared five minute budget.",
    workspaceId,
    templateId: "bug-fix",
    workerPackIds: ["pack:fullstack", "pack:qa"],
    settings: { concurrency: 1, retryLimit: 0, timeoutMinutes: 5, failurePolicy: "stop" },
  }, { dataDir, now: () => "2026-07-20T11:00:00.000Z", teamId: "team:77777777-7777-4777-8777-777777777777" });
  await startAgentTeam(timeoutTeam.teamId, { dataDir, now: () => "2026-07-20T11:00:01.000Z", providerAvailable: true, applyControl: fakeControl() });
  state = await advanceAgentTeam(timeoutTeam.teamId, { dataDir, now: () => "2026-07-20T11:06:00.000Z", providerAvailable: true, applyControl: fakeControl() });
  assert.equal(state.status, "timed_out");
  assert.equal(state.tasks.every((task) => task.status === "cancelled"), true);
});

test("continue policy keeps independent work alive after a sibling failure", async () => {
  const { dataDir, workspaceId } = await setup("patchfleet-team-continue");
  const now = clock();
  const team = await createAgentTeam({
    name: "Independent failure",
    goal: "Keep one independent planning task alive after its sibling fails.",
    workspaceId,
    templateId: "product-feature",
    workerPackIds: ["pack:product", "pack:research", "pack:qa"],
    settings: { concurrency: 2, retryLimit: 0, timeoutMinutes: 30, failurePolicy: "continue" },
  }, { dataDir, now, teamId: "team:88888888-8888-4888-8888-888888888888" });
  let calls = 0;
  const mixedControl = (intent, options) => fakeControl(calls++ === 0 ? "failed" : "running")(intent, options);
  const state = await startAgentTeam(team.teamId, { dataDir, now, providerAvailable: true, applyControl: mixedControl });
  assert.equal(state.tasks.some((task) => task.status === "failed"), true);
  assert.equal(state.tasks.some((task) => task.status === "running"), true);
  assert.equal(state.status, "active");
});
