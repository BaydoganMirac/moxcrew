import { projectWorkEvents } from "./work.mjs";
import { validateAgentPack } from "./agent-pack.mjs";

const UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";
const TEAM_ID = new RegExp(`^team:${UUID}$`, "i");
const AGENT_ID = new RegExp(`^agent:${UUID}$`, "i");
const TASK_ID = new RegExp(`^task:${UUID}$`, "i");
const QUESTION_ID = new RegExp(`^question:${UUID}$`, "i");
const WORKSPACE_ID = new RegExp(`^workspace:${UUID}$`, "i");
const TEMPLATES = new Set(["product-feature", "bug-fix", "saas-launch", "design-frontend", "security-audit", "release"]);
const FAILURE_POLICIES = new Set(["stop", "continue"]);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`invalid ${label}`);
  return value;
}

function exact(value, fields, label) {
  const keys = Object.keys(record(value, label));
  if (keys.length !== fields.length || keys.some((key) => !fields.includes(key))) throw new TypeError(`invalid ${label}`);
}

function text(value, label, maximum) {
  if (
    typeof value !== "string" || value !== value.trim() || value.length < 1 || value.length > maximum ||
    /[\0\u0001-\u0008\u000b\u000c\u000e-\u001f\u007f]/.test(value)
  ) throw new TypeError(`invalid ${label}`);
  return value;
}

function iso(value, label) {
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError(`invalid ${label}`);
  return value;
}

function id(value, pattern, label) {
  if (typeof value !== "string" || !pattern.test(value)) throw new TypeError(`invalid ${label}`);
  return value;
}

function validateQuestion(value) {
  if (value === null) return null;
  exact(value, ["questionId", "prompt", "maxAnswerLength"], "team question");
  if (!Number.isSafeInteger(value.maxAnswerLength) || value.maxAnswerLength < 1 || value.maxAnswerLength > 2000) {
    throw new TypeError("invalid team question limit");
  }
  return {
    questionId: id(value.questionId, QUESTION_ID, "question id"),
    prompt: text(value.prompt, "question prompt", 500),
    maxAnswerLength: value.maxAnswerLength,
  };
}

export function agentTeamWorkItemId(teamId, taskId, attempt) {
  id(teamId, TEAM_ID, "team id");
  id(taskId, TASK_ID, "task id");
  if (!Number.isSafeInteger(attempt) || attempt < 1 || attempt > 6) throw new TypeError("invalid task attempt");
  return `work:${teamId.slice(5)}:${taskId.slice(5)}:${attempt}`;
}

export function validateAgentTeam(value) {
  const input = record(value, "agent team");
  exact(input, [
    "schemaVersion", "teamId", "name", "goal", "workspaceId", "templateId",
    "orchestratorAgentId", "agents", "tasks", "settings", "createdAt",
  ], "agent team");
  if (
    input.schemaVersion !== 1 || !TEMPLATES.has(input.templateId) ||
    !Array.isArray(input.agents) || input.agents.length < 2 || input.agents.length > 12 ||
    !Array.isArray(input.tasks) || input.tasks.length < 1 || input.tasks.length > 32
  ) throw new TypeError("invalid agent team");
  const agents = input.agents.map((agent) => {
    exact(agent, ["agentId", "pack"], "team agent");
    return { agentId: id(agent.agentId, AGENT_ID, "agent id"), pack: validateAgentPack(agent.pack) };
  });
  const agentIds = new Set(agents.map((agent) => agent.agentId));
  if (agentIds.size !== agents.length || !agentIds.has(input.orchestratorAgentId)) throw new TypeError("invalid team agents");
  const orchestrator = agents.find((agent) => agent.agentId === input.orchestratorAgentId);
  if (orchestrator.pack.role !== "orchestrator") throw new TypeError("invalid team orchestrator");
  const tasks = input.tasks.map((task) => {
    exact(task, ["taskId", "title", "instruction", "agentId", "dependsOn", "approvalRequired", "question"], "team task");
    if (!agentIds.has(task.agentId) || task.agentId === input.orchestratorAgentId || !Array.isArray(task.dependsOn) || task.dependsOn.length > 31 || new Set(task.dependsOn).size !== task.dependsOn.length || typeof task.approvalRequired !== "boolean") {
      throw new TypeError("invalid team task");
    }
    return {
      taskId: id(task.taskId, TASK_ID, "task id"),
      title: text(task.title, "task title", 100),
      instruction: text(task.instruction, "task instruction", 4000),
      agentId: task.agentId,
      dependsOn: task.dependsOn.map((dependency) => id(dependency, TASK_ID, "task dependency")),
      approvalRequired: task.approvalRequired,
      question: validateQuestion(task.question),
    };
  });
  const taskIds = new Set(tasks.map((task) => task.taskId));
  if (taskIds.size !== tasks.length || tasks.some((task) => task.dependsOn.some((dependency) => !taskIds.has(dependency) || dependency === task.taskId))) {
    throw new TypeError("invalid team task graph");
  }
  const visiting = new Set();
  const visited = new Set();
  const byId = new Map(tasks.map((task) => [task.taskId, task]));
  function visit(taskId) {
    if (visiting.has(taskId)) throw new TypeError("team task graph contains a cycle");
    if (visited.has(taskId)) return;
    visiting.add(taskId);
    for (const dependency of byId.get(taskId).dependsOn) visit(dependency);
    visiting.delete(taskId);
    visited.add(taskId);
  }
  for (const task of tasks) visit(task.taskId);

  exact(input.settings, ["concurrency", "retryLimit", "timeoutMinutes", "failurePolicy"], "team settings");
  if (
    !Number.isSafeInteger(input.settings.concurrency) || input.settings.concurrency < 1 || input.settings.concurrency > 4 ||
    !Number.isSafeInteger(input.settings.retryLimit) || input.settings.retryLimit < 0 || input.settings.retryLimit > 3 ||
    !Number.isSafeInteger(input.settings.timeoutMinutes) || input.settings.timeoutMinutes < 5 || input.settings.timeoutMinutes > 480 ||
    !FAILURE_POLICIES.has(input.settings.failurePolicy)
  ) throw new TypeError("invalid team settings");
  return {
    schemaVersion: 1,
    teamId: id(input.teamId, TEAM_ID, "team id"),
    name: text(input.name, "team name", 80),
    goal: text(input.goal, "team goal", 10_000),
    workspaceId: id(input.workspaceId, WORKSPACE_ID, "workspace id"),
    templateId: input.templateId,
    orchestratorAgentId: input.orchestratorAgentId,
    agents,
    tasks,
    settings: { ...input.settings },
    createdAt: iso(input.createdAt, "team creation time"),
  };
}

export function validateAgentTeamEvent(value) {
  const input = record(value, "agent team event");
  const payload = record(input.payload, "agent team event payload");
  if (input.type === "team.created") {
    exact(payload, ["team"], "team created payload");
    return { ...input, payload: { team: validateAgentTeam(payload.team) } };
  }
  if (input.type === "team.started" || input.type === "team.cancelled" || input.type === "team.timed_out") {
    exact(payload, ["teamId"], `${input.type} payload`);
    return { ...input, payload: { teamId: id(payload.teamId, TEAM_ID, "team id") } };
  }
  if (input.type === "team.agent.cancelled") {
    exact(payload, ["teamId", "agentId"], "team agent cancelled payload");
    return { ...input, payload: { teamId: id(payload.teamId, TEAM_ID, "team id"), agentId: id(payload.agentId, AGENT_ID, "agent id") } };
  }
  if (input.type === "team.task.retry_requested") {
    exact(payload, ["teamId", "taskId", "previousAttempt"], "team retry payload");
    if (!Number.isSafeInteger(payload.previousAttempt) || payload.previousAttempt < 1 || payload.previousAttempt > 5) throw new TypeError("invalid retry attempt");
    return { ...input, payload: { teamId: id(payload.teamId, TEAM_ID, "team id"), taskId: id(payload.taskId, TASK_ID, "task id"), previousAttempt: payload.previousAttempt } };
  }
  if (input.type === "team.approval.decided") {
    exact(payload, ["teamId", "taskId", "decision", "note"], "team approval payload");
    if (!["approved", "rejected"].includes(payload.decision)) throw new TypeError("invalid approval decision");
    return { ...input, payload: {
      teamId: id(payload.teamId, TEAM_ID, "team id"),
      taskId: id(payload.taskId, TASK_ID, "task id"),
      decision: payload.decision,
      note: payload.note === null ? null : text(payload.note, "approval note", 1000),
    } };
  }
  if (input.type === "team.question.answered") {
    exact(payload, ["teamId", "questionId", "answer"], "team answer payload");
    return { ...input, payload: {
      teamId: id(payload.teamId, TEAM_ID, "team id"),
      questionId: id(payload.questionId, QUESTION_ID, "question id"),
      answer: text(payload.answer, "question answer", 2000),
    } };
  }
  throw new TypeError("unsupported agent team event");
}

function derivedTeam(team, state, work) {
  const workItems = new Map((work?.items ?? []).map((item) => [item.workItemId, item]));
  const runs = new Map((work?.runs ?? []).map((run) => [run.workItemId, run]));
  const taskMap = new Map();
  const definitions = new Map(team.tasks.map((task) => [task.taskId, task]));
  function deriveTask(task) {
    if (taskMap.has(task.taskId)) return taskMap.get(task.taskId);
    const retries = state.retries.get(task.taskId) ?? 0;
    const attempt = retries + 1;
    const workItemId = agentTeamWorkItemId(team.teamId, task.taskId, attempt);
    const item = workItems.get(workItemId);
    const agentCancelled = state.cancelledAgents.has(task.agentId);
    const dependencies = task.dependsOn.map((dependency) => deriveTask(definitions.get(dependency)));
    let status;
    if (state.cancelled || state.timedOut || agentCancelled) status = "cancelled";
    else if (dependencies.some((dependency) => ["failed", "blocked", "cancelled"].includes(dependency?.status))) status = "blocked";
    else if (dependencies.some((dependency) => dependency?.status !== "completed")) status = "pending";
    else if (task.question && !state.answers.has(task.question.questionId)) status = "waiting_question";
    else if (task.approvalRequired && !state.approvals.has(task.taskId)) status = "waiting_approval";
    else if (state.approvals.get(task.taskId)?.decision === "rejected") status = "cancelled";
    else status = item?.status ?? "ready";
    const pack = team.agents.find((agent) => agent.agentId === task.agentId).pack;
    const maxAttempts = Math.min(pack.limits.maxAttempts, team.settings.retryLimit + 1);
    const result = {
      ...task,
      status,
      attempt,
      maxAttempts,
      canRetry: ["failed", "interrupted"].includes(status) && attempt < maxAttempts,
      workItemId: item ? workItemId : null,
      runId: runs.get(workItemId)?.runId ?? null,
      approval: state.approvals.get(task.taskId) ?? null,
      answer: task.question ? state.answers.get(task.question.questionId) ?? null : null,
    };
    taskMap.set(task.taskId, result);
    return result;
  }
  const tasks = team.tasks.map(deriveTask);
  let status = "draft";
  if (state.timedOut) status = "timed_out";
  else if (state.cancelled) status = "cancelled";
  else if (state.startedAt) {
    if (tasks.every((task) => task.status === "completed")) status = "completed";
    else if (tasks.some((task) => task.canRetry)) status = "active";
    else if (tasks.every((task) => ["completed", "failed", "blocked", "cancelled"].includes(task.status))) status = "failed";
    else if (team.settings.failurePolicy === "stop" && tasks.some((task) => ["failed", "blocked", "cancelled"].includes(task.status))) status = "failed";
    else if (tasks.some((task) => ["waiting_question", "waiting_approval"].includes(task.status))) status = "waiting";
    else status = "active";
  }
  const agents = team.agents.map((agent) => {
    const assigned = tasks.filter((task) => task.agentId === agent.agentId);
    return {
      ...agent,
      status: agent.agentId === team.orchestratorAgentId
        ? status
        : state.cancelledAgents.has(agent.agentId)
          ? "cancelled"
          : assigned.some((task) => ["running", "launching", "cancelling"].includes(task.status))
            ? "running"
            : assigned.every((task) => task.status === "completed")
              ? "completed"
              : assigned.some((task) => ["failed", "blocked"].includes(task.status))
                ? "failed"
                : "waiting",
    };
  });
  const derivedRevision = state.revision + tasks.reduce((sum, task) => sum + (task.workItemId ? workItems.get(task.workItemId)?.revision ?? 0 : 0), 0);
  return { ...team, revision: derivedRevision, status, startedAt: state.startedAt, cancelledAt: state.cancelledAt, timedOutAt: state.timedOutAt, agents, tasks };
}

export function projectAgentTeamEvents(events) {
  const teams = new Map();
  let revision = 0;
  let found = false;
  for (const candidate of events) {
    if (!candidate.type.startsWith("team.")) continue;
    found = true;
    const event = validateAgentTeamEvent(candidate);
    if (event.type === "team.created") {
      const team = event.payload.team;
      if (teams.has(team.teamId)) throw new TypeError("duplicate team");
      teams.set(team.teamId, { team, revision: 1, startedAt: null, cancelledAt: null, timedOutAt: null, cancelled: false, timedOut: false, cancelledAgents: new Set(), retries: new Map(), approvals: new Map(), answers: new Map() });
    } else {
      const state = teams.get(event.payload.teamId);
      if (!state) throw new TypeError("team event has no team");
      const { team } = state;
      if (event.type === "team.started") {
        if (state.startedAt || state.cancelled) throw new TypeError("invalid team start");
        state.startedAt = event.recordedAt;
      } else if (event.type === "team.cancelled") {
        if (state.cancelled || state.timedOut) throw new TypeError("duplicate team cancellation");
        state.cancelled = true;
        state.cancelledAt = event.recordedAt;
      } else if (event.type === "team.timed_out") {
        if (state.cancelled || state.timedOut || !state.startedAt) throw new TypeError("invalid team timeout");
        state.timedOut = true;
        state.timedOutAt = event.recordedAt;
      } else if (event.type === "team.agent.cancelled") {
        if (!team.agents.some((agent) => agent.agentId === event.payload.agentId) || event.payload.agentId === team.orchestratorAgentId || state.cancelledAgents.has(event.payload.agentId)) throw new TypeError("invalid agent cancellation");
        state.cancelledAgents.add(event.payload.agentId);
      } else if (event.type === "team.task.retry_requested") {
        const task = team.tasks.find((item) => item.taskId === event.payload.taskId);
        const current = state.retries.get(event.payload.taskId) ?? 0;
        const pack = team.agents.find((agent) => agent.agentId === task?.agentId)?.pack;
        if (!task || event.payload.previousAttempt !== current + 1 || current >= Math.min(team.settings.retryLimit, pack.limits.maxAttempts - 1)) throw new TypeError("invalid task retry");
        state.retries.set(event.payload.taskId, current + 1);
      } else if (event.type === "team.approval.decided") {
        const task = team.tasks.find((item) => item.taskId === event.payload.taskId);
        if (!task?.approvalRequired || state.approvals.has(task.taskId)) throw new TypeError("invalid team approval");
        state.approvals.set(task.taskId, { decision: event.payload.decision, note: event.payload.note, decidedAt: event.recordedAt });
      } else {
        const task = team.tasks.find((item) => item.question?.questionId === event.payload.questionId);
        if (!task || state.answers.has(event.payload.questionId) || event.payload.answer.length > task.question.maxAnswerLength) throw new TypeError("invalid team answer");
        state.answers.set(event.payload.questionId, { answer: event.payload.answer, answeredAt: event.recordedAt });
      }
      state.revision += 1;
      revision += 1;
    }
  }
  if (!found) return null;
  const work = projectWorkEvents(events);
  const items = [...teams.values()].map((state) => derivedTeam(state.team, state, work));
  for (const item of items) {
    for (const task of item.tasks.filter((task) => task.attempt > 1)) {
      for (let attempt = 1; attempt < task.attempt; attempt += 1) {
        const previous = work?.items.find((workItem) => workItem.workItemId === agentTeamWorkItemId(item.teamId, task.taskId, attempt));
        if (!previous || !["failed", "interrupted"].includes(previous.status)) throw new TypeError("retry has no failed attempt");
      }
    }
  }
  return { schemaVersion: 1, revision, items: items.sort((left, right) => right.createdAt.localeCompare(left.createdAt) || left.teamId.localeCompare(right.teamId)) };
}

export function validateAgentTeamProjection(value) {
  const input = record(value, "agent team projection");
  exact(input, ["schemaVersion", "revision", "items"], "agent team projection");
  if (input.schemaVersion !== 1 || !Number.isSafeInteger(input.revision) || input.revision < 0 || !Array.isArray(input.items)) throw new TypeError("invalid agent team projection");
  for (const item of input.items) {
    validateAgentTeam(Object.fromEntries(Object.entries(item).filter(([key]) => !["revision", "status", "startedAt", "cancelledAt", "timedOutAt"].includes(key)).map(([key, value]) => [key, key === "agents" ? value.map(({ status: _status, ...agent }) => agent) : key === "tasks" ? value.map(({ status: _status, attempt: _attempt, maxAttempts: _maxAttempts, canRetry: _canRetry, workItemId: _workItemId, runId: _runId, approval: _approval, answer: _answer, ...task }) => task) : value])));
  }
  return input;
}
