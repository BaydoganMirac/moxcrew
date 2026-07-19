import { createHash } from "node:crypto";
import { validateCommandIntent, validateCommandReceipt } from "../domain/work.mjs";
import { buildCloudProjection } from "./protocol.mjs";

const OPAQUE = /^[A-Za-z0-9._:-]{1,256}$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const TEAM_TYPES = new Set(["cancel_team", "cancel_agent", "approve_decision", "reject_decision", "answer_question", "start_team"]);
const TEAM_STATES = new Set(["draft", "active", "waiting", "completed", "failed", "cancelled", "timed_out"]);
const AGENT_STATES = new Set(["draft", "active", "waiting", "running", "completed", "failed", "cancelled", "timed_out"]);
const TASK_STATES = new Set(["pending", "ready", "waiting_question", "waiting_approval", "queued", "launching", "running", "cancelling", "blocked", "completed", "failed", "interrupted", "cancelled"]);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`${label} must be an object`);
  return value;
}

function exact(value, fields, label) {
  const keys = Object.keys(record(value, label));
  if (keys.length !== fields.length || keys.some((key) => !fields.includes(key))) throw new TypeError(`${label} fields must match`);
}

function opaque(value, label) {
  if (typeof value !== "string" || !OPAQUE.test(value)) throw new TypeError(`${label} is invalid`);
  return value;
}

function integer(value, label, minimum = 0, maximum = Number.MAX_SAFE_INTEGER) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) throw new TypeError(`${label} is invalid`);
  return value;
}

function iso(value, label) {
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError(`${label} is invalid`);
  return value;
}

function text(value, label, maximum) {
  if (typeof value !== "string" || value !== value.trim() || value.length < 1 || value.length > maximum || /[\0\u0001-\u0008\u000b\u000c\u000e-\u001f\u007f]/.test(value)) throw new TypeError(`${label} is invalid`);
  return value;
}

export function cloudWorkspaceAlias(installationId, workspaceId) {
  opaque(installationId, "installation id");
  opaque(workspaceId, "workspace id");
  return `workspace-alias:${createHash("sha256").update(`${installationId}\0${workspaceId}`).digest("hex")}`;
}

export function buildCloudProjectionV2({ observation, work, teams, workspaces, agentPacks, revision, installationId }) {
  const base = buildCloudProjection(observation, work, revision);
  const items = (teams?.items ?? []).slice(0, 16).map((team) => ({
    teamId: opaque(team.teamId, "team id"),
    workspaceAlias: cloudWorkspaceAlias(installationId, team.workspaceId),
    templateId: text(team.templateId, "template id", 40),
    status: TEAM_STATES.has(team.status) ? team.status : (() => { throw new TypeError("invalid team status"); })(),
    revision: integer(team.revision, "team revision", 1),
    settings: {
      concurrency: integer(team.settings.concurrency, "team concurrency", 1, 4),
      retryLimit: integer(team.settings.retryLimit, "team retry limit", 0, 3),
      timeoutMinutes: integer(team.settings.timeoutMinutes, "team timeout", 5, 480),
      failurePolicy: ["stop", "continue"].includes(team.settings.failurePolicy) ? team.settings.failurePolicy : (() => { throw new TypeError("invalid failure policy"); })(),
    },
    agents: team.agents.map((agent) => ({
      agentId: opaque(agent.agentId, "agent id"),
      packId: opaque(agent.pack.id, "pack id"),
      role: text(agent.pack.role, "agent role", 40),
      status: AGENT_STATES.has(agent.status) ? agent.status : (() => { throw new TypeError("invalid agent status"); })(),
    })),
    tasks: team.tasks.map((task) => ({
      taskId: opaque(task.taskId, "task id"),
      agentId: opaque(task.agentId, "agent id"),
      status: TASK_STATES.has(task.status) ? task.status : (() => { throw new TypeError("invalid task status"); })(),
      attempt: integer(task.attempt, "task attempt", 1, 6),
      maxAttempts: integer(task.maxAttempts, "task max attempts", 1, 6),
      approvalRequired: typeof task.approvalRequired === "boolean" ? task.approvalRequired : (() => { throw new TypeError("invalid approval flag"); })(),
      question: task.question && task.status === "waiting_question"
        ? { questionId: opaque(task.question.questionId, "question id"), kind: "release_constraints", maxAnswerLength: integer(task.question.maxAnswerLength, "answer limit", 1, 2000) }
        : null,
    })),
  }));
  return {
    ...base,
    protocolVersion: 2,
    capabilities: {
      teamSupervision: true,
      remoteTeamStart: true,
      actions: ["cancel_run", "cancel_team", "cancel_agent", "approve_decision", "reject_decision", "answer_question", "start_team"],
    },
    workspaces: (workspaces?.items ?? []).slice(0, 32).map((workspace) => ({ workspaceAlias: cloudWorkspaceAlias(installationId, workspace.workspaceId) })),
    agentPacks: (agentPacks ?? []).slice(0, 32).map((pack) => ({ packId: opaque(pack.id, "pack id"), role: text(pack.role, "pack role", 40) })),
    teams: items,
    teamsTruncated: (teams?.items.length ?? 0) > 16,
  };
}

function teamIntent(value) {
  exact(value, ["schemaVersion", "intentId", "idempotencyKey", "type", "actorId", "createdAt", "expiresAt", "payload"], "team intent");
  if (value.schemaVersion !== 2 || !TEAM_TYPES.has(value.type) || value.actorId !== "cloud-owner" || !UUID.test(value.intentId)) throw new TypeError("invalid team intent");
  const base = {
    schemaVersion: 2,
    intentId: value.intentId,
    idempotencyKey: opaque(value.idempotencyKey, "idempotency key"),
    type: value.type,
    actorId: "cloud-owner",
    createdAt: iso(value.createdAt, "created at"),
    expiresAt: iso(value.expiresAt, "expires at"),
  };
  if (value.type === "cancel_team") {
    exact(value.payload, ["teamId", "expectedTeamRevision"], "cancel team payload");
    return { ...base, payload: { teamId: opaque(value.payload.teamId, "team id"), expectedTeamRevision: integer(value.payload.expectedTeamRevision, "team revision", 1) } };
  }
  if (value.type === "cancel_agent") {
    exact(value.payload, ["teamId", "agentId", "expectedTeamRevision"], "cancel agent payload");
    return { ...base, payload: { teamId: opaque(value.payload.teamId, "team id"), agentId: opaque(value.payload.agentId, "agent id"), expectedTeamRevision: integer(value.payload.expectedTeamRevision, "team revision", 1) } };
  }
  if (["approve_decision", "reject_decision"].includes(value.type)) {
    exact(value.payload, ["teamId", "taskId", "expectedTeamRevision", "note"], "decision payload");
    return { ...base, payload: { teamId: opaque(value.payload.teamId, "team id"), taskId: opaque(value.payload.taskId, "task id"), expectedTeamRevision: integer(value.payload.expectedTeamRevision, "team revision", 1), note: value.payload.note === null ? null : text(value.payload.note, "decision note", 1000) } };
  }
  if (value.type === "answer_question") {
    exact(value.payload, ["teamId", "questionId", "expectedTeamRevision", "answer"], "answer payload");
    return { ...base, payload: { teamId: opaque(value.payload.teamId, "team id"), questionId: opaque(value.payload.questionId, "question id"), expectedTeamRevision: integer(value.payload.expectedTeamRevision, "team revision", 1), answer: text(value.payload.answer, "answer", 2000) } };
  }
  exact(value.payload, ["workspaceAlias", "teamName", "goal", "templateId", "orchestratorPackId", "workerPackIds", "settings"], "start team payload");
  exact(value.payload.settings, ["concurrency", "retryLimit", "timeoutMinutes", "failurePolicy"], "start team settings");
  if (!Array.isArray(value.payload.workerPackIds) || value.payload.workerPackIds.length < 1 || value.payload.workerPackIds.length > 4 || new Set(value.payload.workerPackIds).size !== value.payload.workerPackIds.length) throw new TypeError("invalid worker packs");
  return { ...base, payload: {
    workspaceAlias: opaque(value.payload.workspaceAlias, "workspace alias"),
    teamName: text(value.payload.teamName, "team name", 80),
    goal: text(value.payload.goal, "team goal", 4000),
    templateId: text(value.payload.templateId, "template id", 40),
    orchestratorPackId: opaque(value.payload.orchestratorPackId, "orchestrator pack id"),
    workerPackIds: value.payload.workerPackIds.map((item) => opaque(item, "worker pack id")),
    settings: {
      concurrency: integer(value.payload.settings.concurrency, "concurrency", 1, 4),
      retryLimit: integer(value.payload.settings.retryLimit, "retry limit", 0, 3),
      timeoutMinutes: integer(value.payload.settings.timeoutMinutes, "timeout", 5, 480),
      failurePolicy: ["stop", "continue"].includes(value.payload.settings.failurePolicy) ? value.payload.settings.failurePolicy : (() => { throw new TypeError("invalid failure policy"); })(),
    },
  } };
}

export function validateRemoteTeamIntent(value) {
  const intent = teamIntent(value);
  if (intent.expiresAt <= intent.createdAt || Date.parse(intent.expiresAt) - Date.parse(intent.createdAt) > 5 * 60_000) throw new TypeError("invalid team intent window");
  return intent;
}

export function validateIntentPageV2(value, hostId, after, now) {
  exact(value, ["schemaVersion", "hostId", "cursor", "intents"], "intent page");
  if (value.schemaVersion !== 2 || opaque(value.hostId, "host id") !== hostId || !Array.isArray(value.intents) || value.intents.length > 20) throw new TypeError("invalid intent page");
  let cursor = integer(after, "after");
  const current = Date.parse(iso(now, "host now"));
  const intents = value.intents.map((entry) => {
    exact(entry, ["sequence", "intent"], "sequenced intent");
    const sequence = integer(entry.sequence, "intent sequence", 1);
    if (sequence <= cursor) throw new TypeError("intent sequence replay or reordering");
    cursor = sequence;
    const intent = entry.intent.schemaVersion === 1 ? validateCommandIntent(entry.intent) : validateRemoteTeamIntent(entry.intent);
    if ((intent.schemaVersion === 1 && (intent.type !== "cancel_run" || intent.actorId !== "cloud-owner")) || Date.parse(intent.createdAt) - current > 60_000) throw new TypeError("unsupported remote intent");
    return { sequence, intent };
  });
  if (integer(value.cursor, "cursor") !== cursor) throw new TypeError("intent cursor mismatch");
  return { schemaVersion: 2, hostId, cursor, intents };
}

export function buildRemoteReceiptPayload(receipt) {
  if (receipt.schemaVersion === 1) return { receipts: [validateCommandReceipt(receipt)] };
  exact(receipt, ["schemaVersion", "intentId", "idempotencyKey", "commandType", "outcome", "reasonCode", "completedAt", "teamId", "teamRevision"], "team receipt");
  if (receipt.schemaVersion !== 2 || !TEAM_TYPES.has(receipt.commandType) || !["applied", "rejected", "expired", "failed"].includes(receipt.outcome)) throw new TypeError("invalid team receipt");
  return { receipts: [{ ...receipt, intentId: opaque(receipt.intentId, "intent id"), idempotencyKey: opaque(receipt.idempotencyKey, "idempotency key"), completedAt: iso(receipt.completedAt, "completed at"), teamId: receipt.teamId === null ? null : opaque(receipt.teamId, "team id"), teamRevision: integer(receipt.teamRevision, "team revision") }] };
}
