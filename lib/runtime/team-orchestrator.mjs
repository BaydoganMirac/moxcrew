import { randomUUID } from "node:crypto";
import { TEAM_TEMPLATES, teamTaskCopy, templatePhase } from "../agents/team-templates.mjs";
import { agentTeamWorkItemId, projectAgentTeamEvents, validateAgentTeam } from "../domain/agent-team.mjs";
import { supportsCodexControl } from "../providers/codex.mjs";
import { commitEventTransaction, readAgentTeamProjection, readProjection, readWorkProjection } from "./observation-store.mjs";
import { listAgentPacks } from "./agent-pack-registry.mjs";
import { applyWorkCommand, applyWorkControlCommand } from "./work-queue.mjs";
import { resolveRegisteredWorkspace } from "./workspace-registry.mjs";

export { readAgentTeamProjection };

function event(type, recordedAt, payload) {
  return { id: randomUUID(), schemaVersion: 1, type, recordedAt, payload };
}

function timestamp(now) {
  const value = now();
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError("now must return an ISO timestamp");
  return value;
}

function commandBase(now, type) {
  const commandId = `cmd:${randomUUID()}`;
  return {
    schemaVersion: 1,
    intentId: commandId,
    idempotencyKey: commandId,
    type,
    actorId: "local-orchestrator",
    createdAt: now,
    expiresAt: new Date(new Date(now).valueOf() + 5 * 60_000).toISOString(),
  };
}

function teamInputText(value, label, maximum) {
  const normalized = typeof value === "string" ? value.trim() : "";
  if (!normalized || normalized.length > maximum) throw new TypeError(`invalid ${label}`);
  return normalized;
}

export async function createAgentTeam({
  name,
  goal,
  workspaceId,
  templateId,
  orchestratorPackId = "pack:orchestrator",
  workerPackIds,
  settings = {},
}, {
  dataDir,
  now = () => new Date().toISOString(),
  teamId = `team:${randomUUID()}`,
} = {}) {
  if (!TEAM_TEMPLATES.some((template) => template.id === templateId)) throw new TypeError("unknown team template");
  if (!Array.isArray(workerPackIds) || workerPackIds.length < 1 || workerPackIds.length > 11 || new Set(workerPackIds).size !== workerPackIds.length || workerPackIds.includes(orchestratorPackId)) {
    throw new TypeError("select one to eleven unique worker packs");
  }
  if (!await resolveRegisteredWorkspace(workspaceId, { dataDir })) throw new TypeError("registered workspace is required");
  const catalog = await listAgentPacks({ dataDir });
  const selected = [orchestratorPackId, ...workerPackIds].map((packId) => {
    const pack = catalog.find((candidate) => candidate.id === packId);
    if (!pack) throw new TypeError(`agent pack is not installed: ${packId}`);
    return pack;
  });
  if (selected[0].role !== "orchestrator") throw new TypeError("orchestrator pack must use the orchestrator role");
  const agents = selected.map((pack) => ({ agentId: `agent:${randomUUID()}`, pack }));
  const workers = agents.slice(1).map((agent) => ({ ...agent, phase: templatePhase(templateId, agent.pack.role) })).sort((left, right) => left.phase - right.phase || left.pack.name.localeCompare(right.pack.name));
  const taskSeeds = workers.map((agent) => ({ agent, taskId: `task:${randomUUID()}`, ...teamTaskCopy(templateId, agent.pack) }));
  const phases = [...new Set(taskSeeds.map((seed) => seed.agent.phase))];
  const tasks = taskSeeds.map((seed) => {
    const phaseIndex = phases.indexOf(seed.agent.phase);
    const previousPhase = phaseIndex > 0 ? phases[phaseIndex - 1] : null;
    return {
      taskId: seed.taskId,
      title: seed.title,
      instruction: seed.instruction,
      agentId: seed.agent.agentId,
      dependsOn: previousPhase === null ? [] : taskSeeds.filter((candidate) => candidate.agent.phase === previousPhase).map((candidate) => candidate.taskId),
      approvalRequired: seed.approvalRequired,
      question: seed.asksReleaseQuestion ? {
        questionId: `question:${randomUUID()}`,
        prompt: "Confirm the release target and any constraints the release agent must follow.",
        maxAnswerLength: 1000,
      } : null,
    };
  });
  const createdAt = timestamp(now);
  const team = validateAgentTeam({
    schemaVersion: 1,
    teamId,
    name: teamInputText(name, "team name", 80),
    goal: teamInputText(goal, "team goal", 10_000),
    workspaceId,
    templateId,
    orchestratorAgentId: agents[0].agentId,
    agents: agents.map(({ agentId, pack }) => ({ agentId, pack })),
    tasks,
    settings: {
      concurrency: settings.concurrency ?? 2,
      retryLimit: settings.retryLimit ?? 1,
      timeoutMinutes: settings.timeoutMinutes ?? 120,
      failurePolicy: settings.failurePolicy ?? "stop",
    },
    createdAt,
  });
  return commitEventTransaction((events) => {
    if (projectAgentTeamEvents(events)?.items.some((item) => item.teamId === team.teamId)) throw new TypeError("team already exists");
    return { additions: [event("team.created", createdAt, { team })], result: () => team };
  }, { dataDir });
}

async function appendTeamEvent(type, payload, { dataDir, now = () => new Date().toISOString(), check }) {
  const recordedAt = timestamp(now);
  return commitEventTransaction((events) => {
    const projection = projectAgentTeamEvents(events);
    const team = projection?.items.find((item) => item.teamId === payload.teamId);
    if (!team || (check && !check(team))) throw new TypeError("team action is not available");
    return { additions: [event(type, recordedAt, payload)], result: () => recordedAt };
  }, { dataDir });
}

export async function startAgentTeam(teamId, options = {}) {
  await appendTeamEvent("team.started", { teamId }, { ...options, check: (team) => team.status === "draft" });
  return advanceAgentTeam(teamId, options);
}

export async function answerTeamQuestion(teamId, questionId, answer, options = {}) {
  const normalized = teamInputText(answer, "question answer", 2000);
  await appendTeamEvent("team.question.answered", { teamId, questionId, answer: normalized }, {
    ...options,
    check: (team) => team.tasks.some((task) => task.question?.questionId === questionId && task.status === "waiting_question" && normalized.length <= task.question.maxAnswerLength),
  });
  return readAgentTeamProjection({ dataDir: options.dataDir });
}

export async function decideTeamApproval(teamId, taskId, decision, note, options = {}) {
  if (!["approved", "rejected"].includes(decision)) throw new TypeError("invalid approval decision");
  const normalizedNote = note === null || note === "" ? null : teamInputText(note, "approval note", 1000);
  await appendTeamEvent("team.approval.decided", { teamId, taskId, decision, note: normalizedNote }, {
    ...options,
    check: (team) => team.tasks.some((task) => task.taskId === taskId && task.status === "waiting_approval"),
  });
  return readAgentTeamProjection({ dataDir: options.dataDir });
}

function taskInstruction(team, task) {
  const agent = team.agents.find((candidate) => candidate.agentId === task.agentId);
  const answer = task.answer?.answer ? `\nOwner answer: ${task.answer.answer}` : "";
  const approval = task.approval?.note ? `\nOwner approval note: ${task.approval.note}` : "";
  return [
    `Patchfleet team: ${team.name}`,
    `Team goal: ${team.goal}`,
    `Assigned task: ${task.title}`,
    task.instruction,
    `Agent contract: ${agent.pack.instructions}`,
    `Declared permissions: ${agent.pack.permissions.join(", ")}. These declarations never widen the provider sandbox.`,
    `Required quality checks: ${agent.pack.qualityChecks.join(", ") || "task-defined"}.`,
    `Expected output: ${agent.pack.expectedOutput}.${answer}${approval}`,
  ].join("\n\n");
}

async function controlAvailable(dataDir) {
  const projection = await readProjection({ dataDir });
  return projection?.observations.some(supportsCodexControl) ?? false;
}

export async function advanceAgentTeam(teamId, {
  dataDir,
  now = () => new Date().toISOString(),
  providerAvailable,
  applyQueue = applyWorkCommand,
  applyControl = applyWorkControlCommand,
} = {}) {
  let projection = await readAgentTeamProjection({ dataDir });
  let team = projection?.items.find((item) => item.teamId === teamId);
  if (!team || !["active", "waiting"].includes(team.status)) return team ?? null;

  const checkedAt = timestamp(now);
  if (new Date(checkedAt).valueOf() >= new Date(team.startedAt).valueOf() + team.settings.timeoutMinutes * 60_000) {
    const available = providerAvailable ?? await controlAvailable(dataDir);
    await appendTeamEvent("team.timed_out", { teamId }, { dataDir, now: () => checkedAt, check: (current) => ["active", "waiting"].includes(current.status) });
    await stopTaskWork(team.tasks, { dataDir, now, providerAvailable: available, applyQueue, applyControl });
    return (await readAgentTeamProjection({ dataDir })).items.find((item) => item.teamId === teamId);
  }

  for (const task of team.tasks.filter((item) => item.canRetry)) {
    await appendTeamEvent("team.task.retry_requested", { teamId, taskId: task.taskId, previousAttempt: task.attempt }, {
      dataDir,
      now,
      check: (current) => current.tasks.some((item) => item.taskId === task.taskId && item.canRetry && item.attempt === task.attempt),
    });
  }
  if (team.tasks.some((task) => task.canRetry)) {
    projection = await readAgentTeamProjection({ dataDir });
    team = projection.items.find((item) => item.teamId === teamId);
  }

  const workspace = await resolveRegisteredWorkspace(team.workspaceId, { dataDir });
  if (!workspace) throw new TypeError("team workspace is no longer registered");
  const available = providerAvailable ?? await controlAvailable(dataDir);
  let active = team.tasks.filter((task) => ["launching", "running", "cancelling"].includes(task.status)).length;
  let slots = Math.max(0, team.settings.concurrency - active);
  const candidates = [
    ...team.tasks.filter((task) => task.status === "queued"),
    ...team.tasks.filter((task) => task.status === "ready"),
  ];
  for (const task of candidates) {
    if (slots < 1) break;
    const recordedAt = timestamp(now);
    let item = (await readWorkProjection({ dataDir }))?.items.find((candidate) => candidate.workItemId === agentTeamWorkItemId(team.teamId, task.taskId, task.attempt));
    if (!item) {
      const intent = commandBase(recordedAt, "enqueue_work");
      await applyQueue({
        ...intent,
        payload: { workItem: {
          schemaVersion: 1,
          workItemId: agentTeamWorkItemId(team.teamId, task.taskId, task.attempt),
          title: `${team.name}: ${task.title}`.slice(0, 160),
          instruction: taskInstruction(team, task),
          providerId: "codex",
          workingDirectory: workspace.workingDirectory,
          status: "queued",
          createdAt: recordedAt,
          revision: 1,
        } },
      }, { dataDir, now: () => recordedAt });
      item = (await readWorkProjection({ dataDir }))?.items.find((candidate) => candidate.workItemId === agentTeamWorkItemId(team.teamId, task.taskId, task.attempt));
    }
    if (item?.status !== "queued") continue;
    const startAt = timestamp(now);
    const receipt = await applyControl({
      ...commandBase(startAt, "start_work"),
      payload: { workItemId: item.workItemId, expectedItemRevision: item.revision },
    }, { dataDir, now: () => startAt, providerAvailable: available });
    if (receipt.outcome === "applied") {
      active += 1;
      slots -= 1;
    }
  }
  return (await readAgentTeamProjection({ dataDir }))?.items.find((item) => item.teamId === teamId) ?? null;
}

async function stopTaskWork(tasks, { dataDir, now, providerAvailable, applyQueue, applyControl }) {
  const work = await readWorkProjection({ dataDir });
  for (const task of tasks) {
    const item = work?.items.find((candidate) => candidate.workItemId === task.workItemId);
    const run = work?.runs.find((candidate) => candidate.runId === task.runId);
    const recordedAt = timestamp(now);
    if (item?.status === "queued") {
      await applyQueue({ ...commandBase(recordedAt, "remove_queued_work"), payload: { workItemId: item.workItemId, expectedItemRevision: item.revision } }, { dataDir, now: () => recordedAt });
    } else if (run && ["running", "cancelling"].includes(run.status)) {
      await applyControl({ ...commandBase(recordedAt, "cancel_run"), payload: { runId: run.runId, expectedRunRevision: run.revision } }, { dataDir, now: () => recordedAt, providerAvailable });
    }
  }
}

export async function cancelAgentTeam(teamId, {
  dataDir,
  now = () => new Date().toISOString(),
  providerAvailable,
  applyQueue = applyWorkCommand,
  applyControl = applyWorkControlCommand,
} = {}) {
  const team = (await readAgentTeamProjection({ dataDir }))?.items.find((item) => item.teamId === teamId);
  if (!team || !["active", "waiting"].includes(team.status)) throw new TypeError("team cannot be cancelled");
  const available = providerAvailable ?? await controlAvailable(dataDir);
  await appendTeamEvent("team.cancelled", { teamId }, { dataDir, now, check: (current) => ["active", "waiting"].includes(current.status) });
  await stopTaskWork(team.tasks, { dataDir, now, providerAvailable: available, applyQueue, applyControl });
  return (await readAgentTeamProjection({ dataDir })).items.find((item) => item.teamId === teamId);
}

export async function cancelTeamAgent(teamId, agentId, {
  dataDir,
  now = () => new Date().toISOString(),
  providerAvailable,
  applyQueue = applyWorkCommand,
  applyControl = applyWorkControlCommand,
} = {}) {
  const team = (await readAgentTeamProjection({ dataDir }))?.items.find((item) => item.teamId === teamId);
  const agent = team?.agents.find((item) => item.agentId === agentId);
  if (!team || !agent || agentId === team.orchestratorAgentId || agent.status === "cancelled") throw new TypeError("agent cannot be cancelled");
  const available = providerAvailable ?? await controlAvailable(dataDir);
  await appendTeamEvent("team.agent.cancelled", { teamId, agentId }, { dataDir, now, check: (current) => current.agents.some((item) => item.agentId === agentId && item.status !== "cancelled") });
  await stopTaskWork(team.tasks.filter((task) => task.agentId === agentId), { dataDir, now, providerAvailable: available, applyQueue, applyControl });
  return (await readAgentTeamProjection({ dataDir })).items.find((item) => item.teamId === teamId);
}

export async function advanceAgentTeams(options = {}) {
  const projection = await readAgentTeamProjection({ dataDir: options.dataDir });
  const results = [];
  for (const team of projection?.items.filter((item) => ["active", "waiting"].includes(item.status)) ?? []) {
    results.push(await advanceAgentTeam(team.teamId, options));
  }
  return results;
}
