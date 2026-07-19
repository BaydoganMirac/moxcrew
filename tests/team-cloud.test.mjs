import assert from "node:assert/strict";
import { mkdir, mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { BUILT_IN_AGENT_PACKS } from "../lib/agents/built-in-packs.mjs";
import { applyRemoteTeamIntent } from "../lib/cloud/team-intents.mjs";
import {
  buildCloudProjectionV2,
  cloudWorkspaceAlias,
  validateIntentPageV2,
} from "../lib/cloud/protocol-v2.mjs";
import { readAgentTeamProjection } from "../lib/runtime/observation-store.mjs";
import { registerWorkspace } from "../lib/runtime/workspace-registry.mjs";

test("V2 team projection is a strict sanitized allowlist", () => {
  const installationId = "install:11111111-1111-4111-8111-111111111111";
  const workspaceId = "workspace:22222222-2222-4222-8222-222222222222";
  const projection = buildCloudProjectionV2({
    observation: null,
    work: null,
    revision: 7,
    installationId,
    workspaces: { items: [{ workspaceId, displayName: "FORBIDDEN_REPOSITORY_NAME", workingDirectory: "/FORBIDDEN/PATH" }] },
    agentPacks: BUILT_IN_AGENT_PACKS,
    teams: { items: [{
      teamId: "team:33333333-3333-4333-8333-333333333333",
      workspaceId,
      templateId: "product-feature",
      status: "waiting",
      revision: 4,
      goal: "FORBIDDEN_GOAL_CANARY",
      settings: { concurrency: 2, retryLimit: 1, timeoutMinutes: 120, failurePolicy: "stop" },
      agents: [
        { agentId: "agent:44444444-4444-4444-8444-444444444444", pack: BUILT_IN_AGENT_PACKS[0], status: "waiting" },
        { agentId: "agent:55555555-5555-4555-8555-555555555555", pack: { ...BUILT_IN_AGENT_PACKS[1], instructions: "FORBIDDEN_PACK_INSTRUCTIONS" }, status: "waiting" },
      ],
      tasks: [{
        taskId: "task:66666666-6666-4666-8666-666666666666",
        agentId: "agent:55555555-5555-4555-8555-555555555555",
        title: "FORBIDDEN_TASK_TITLE",
        instruction: "FORBIDDEN_TASK_INSTRUCTION",
        status: "waiting_question",
        attempt: 1,
        maxAttempts: 2,
        approvalRequired: true,
        question: { questionId: "question:77777777-7777-4777-8777-777777777777", prompt: "FORBIDDEN_QUESTION_PROMPT", maxAnswerLength: 1000 },
      }],
    }] },
  });
  const encoded = JSON.stringify(projection);
  for (const canary of ["FORBIDDEN_REPOSITORY_NAME", "/FORBIDDEN/PATH", "FORBIDDEN_GOAL_CANARY", "FORBIDDEN_PACK_INSTRUCTIONS", "FORBIDDEN_TASK_TITLE", "FORBIDDEN_TASK_INSTRUCTION", "FORBIDDEN_QUESTION_PROMPT"]) {
    assert.equal(encoded.includes(canary), false, canary);
  }
  assert.equal(projection.protocolVersion, 2);
  assert.deepEqual(projection.workspaces, [{ workspaceAlias: cloudWorkspaceAlias(installationId, workspaceId) }]);
  assert.deepEqual(projection.teams[0].tasks[0].question.kind, "release_constraints");
});

test("V2 intent page rejects unknown powers and preserves V1 cancel compatibility", () => {
  const hostId = "host:one";
  const now = "2026-07-20T12:00:00.000Z";
  const start = {
    schemaVersion: 2,
    intentId: "88888888-8888-4888-8888-888888888888",
    idempotencyKey: "remote-start-one",
    type: "start_team",
    actorId: "cloud-owner",
    createdAt: now,
    expiresAt: "2026-07-20T12:05:00.000Z",
    payload: {
      workspaceAlias: "workspace-alias:one",
      teamName: "Remote team",
      goal: "Deliver one bounded tested slice.",
      templateId: "product-feature",
      orchestratorPackId: "pack:orchestrator",
      workerPackIds: ["pack:fullstack"],
      settings: { concurrency: 1, retryLimit: 1, timeoutMinutes: 60, failurePolicy: "stop" },
    },
  };
  assert.equal(validateIntentPageV2({ schemaVersion: 2, hostId, cursor: 1, intents: [{ sequence: 1, intent: start }] }, hostId, 0, now).intents[0].intent.type, "start_team");
  assert.throws(() => validateIntentPageV2({ schemaVersion: 2, hostId, cursor: 1, intents: [{ sequence: 1, intent: { ...start, type: "run_shell" } }] }, hostId, 0, now), /invalid team intent/);
  assert.throws(() => validateIntentPageV2({ schemaVersion: 2, hostId, cursor: 1, intents: [{ sequence: 1, intent: { ...start, payload: { ...start.payload, prompt: "raw" } } }] }, hostId, 0, now), /fields must match/);
});

test("remote team start and cancel are idempotent and resolve only a local workspace alias", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-team-cloud-data-"));
  const workspace = await mkdtemp(join(tmpdir(), "patchfleet-team-cloud-workspace-"));
  await mkdir(join(workspace, ".git"));
  const installationId = "install:99999999-9999-4999-8999-999999999999";
  const workspaceId = "workspace:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
  await registerWorkspace(workspace, { dataDir, workspaceId, commandId: "cmd:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", now: () => "2026-07-20T12:59:00.000Z" });
  const intent = {
    schemaVersion: 2,
    intentId: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
    idempotencyKey: "remote-team-start",
    type: "start_team",
    actorId: "cloud-owner",
    createdAt: "2026-07-20T13:00:00.000Z",
    expiresAt: "2026-07-20T13:05:00.000Z",
    payload: {
      workspaceAlias: cloudWorkspaceAlias(installationId, workspaceId),
      teamName: "Remote product slice",
      goal: "Implement and test one bounded product slice.",
      templateId: "product-feature",
      orchestratorPackId: "pack:orchestrator",
      workerPackIds: ["pack:fullstack"],
      settings: { concurrency: 1, retryLimit: 1, timeoutMinutes: 60, failurePolicy: "stop" },
    },
  };
  const options = { dataDir, installationId, now: () => "2026-07-20T13:00:01.000Z", teamOptions: { providerAvailable: false } };
  const first = await applyRemoteTeamIntent(intent, options);
  const replay = await applyRemoteTeamIntent(intent, options);
  assert.deepEqual([first.outcome, first.reasonCode], ["applied", "TEAM_STARTED"]);
  assert.deepEqual([replay.outcome, replay.reasonCode], ["applied", "TEAM_STARTED"]);
  let team = (await readAgentTeamProjection({ dataDir })).items[0];
  assert.equal((await readAgentTeamProjection({ dataDir })).items.length, 1);
  const cancelled = await applyRemoteTeamIntent({
    schemaVersion: 2,
    intentId: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
    idempotencyKey: "remote-team-cancel",
    type: "cancel_team",
    actorId: "cloud-owner",
    createdAt: "2026-07-20T13:01:00.000Z",
    expiresAt: "2026-07-20T13:06:00.000Z",
    payload: { teamId: team.teamId, expectedTeamRevision: team.revision },
  }, { dataDir, installationId, now: () => "2026-07-20T13:01:01.000Z", teamOptions: { providerAvailable: false } });
  assert.deepEqual([cancelled.outcome, cancelled.reasonCode], ["applied", "TEAM_CANCELLED"]);
  team = (await readAgentTeamProjection({ dataDir })).items[0];
  assert.equal(team.status, "cancelled");
});
