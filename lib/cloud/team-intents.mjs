import { readAgentTeamProjection, readWorkspaceProjection } from "../runtime/observation-store.mjs";
import {
  advanceAgentTeam,
  answerTeamQuestion,
  cancelAgentTeam,
  cancelTeamAgent,
  createAgentTeam,
  decideTeamApproval,
  startAgentTeam,
} from "../runtime/team-orchestrator.mjs";
import { cloudWorkspaceAlias, validateRemoteTeamIntent } from "./protocol-v2.mjs";

function receipt(intent, outcome, reasonCode, completedAt, team) {
  return {
    schemaVersion: 2,
    intentId: intent.intentId,
    idempotencyKey: intent.idempotencyKey,
    commandType: intent.type,
    outcome,
    reasonCode,
    completedAt,
    teamId: team?.teamId ?? (intent.payload.teamId ?? null),
    teamRevision: team?.revision ?? 0,
  };
}

export async function applyRemoteTeamIntent(value, {
  dataDir,
  installationId,
  now = () => new Date().toISOString(),
  teamOptions = {},
} = {}) {
  const intent = validateRemoteTeamIntent(value);
  const completedAt = now();
  if (intent.expiresAt <= completedAt) return receipt(intent, "expired", "COMMAND_EXPIRED", completedAt, null);
  let projection = await readAgentTeamProjection({ dataDir });
  let team = intent.payload.teamId ? projection?.items.find((item) => item.teamId === intent.payload.teamId) : null;
  try {
    if (intent.type === "start_team") {
      const teamId = `team:${intent.intentId}`;
      team = projection?.items.find((item) => item.teamId === teamId);
      if (!team) {
        const workspaces = await readWorkspaceProjection({ dataDir });
        const workspace = workspaces?.items.find((item) => cloudWorkspaceAlias(installationId, item.workspaceId) === intent.payload.workspaceAlias);
        if (!workspace) return receipt(intent, "rejected", "WORKSPACE_ALIAS_NOT_FOUND", completedAt, null);
        await createAgentTeam({
          name: intent.payload.teamName,
          goal: intent.payload.goal,
          workspaceId: workspace.workspaceId,
          templateId: intent.payload.templateId,
          orchestratorPackId: intent.payload.orchestratorPackId,
          workerPackIds: intent.payload.workerPackIds,
          settings: intent.payload.settings,
        }, { dataDir, now: () => completedAt, teamId });
        team = await startAgentTeam(teamId, { dataDir, ...teamOptions });
      }
      return receipt(intent, "applied", "TEAM_STARTED", completedAt, team);
    }
    if (!team) return receipt(intent, "rejected", "TEAM_NOT_FOUND", completedAt, null);
    if (intent.type === "cancel_team" && team.status === "cancelled") return receipt(intent, "applied", "TEAM_CANCELLED", completedAt, team);
    if (intent.type === "cancel_agent" && team.agents.find((agent) => agent.agentId === intent.payload.agentId)?.status === "cancelled") return receipt(intent, "applied", "AGENT_CANCELLED", completedAt, team);
    const task = intent.payload.taskId ? team.tasks.find((item) => item.taskId === intent.payload.taskId) : null;
    if (["approve_decision", "reject_decision"].includes(intent.type) && task?.approval?.decision === (intent.type === "approve_decision" ? "approved" : "rejected")) return receipt(intent, "applied", intent.type === "approve_decision" ? "DECISION_APPROVED" : "DECISION_REJECTED", completedAt, team);
    const questionTask = intent.payload.questionId ? team.tasks.find((item) => item.question?.questionId === intent.payload.questionId) : null;
    if (intent.type === "answer_question" && questionTask?.answer?.answer === intent.payload.answer) return receipt(intent, "applied", "QUESTION_ANSWERED", completedAt, team);
    if (team.revision !== intent.payload.expectedTeamRevision) return receipt(intent, "rejected", "STALE_TEAM_REVISION", completedAt, team);

    if (intent.type === "cancel_team") team = await cancelAgentTeam(team.teamId, { dataDir, ...teamOptions });
    else if (intent.type === "cancel_agent") team = await cancelTeamAgent(team.teamId, intent.payload.agentId, { dataDir, ...teamOptions });
    else if (intent.type === "answer_question") {
      await answerTeamQuestion(team.teamId, intent.payload.questionId, intent.payload.answer, { dataDir });
      team = await advanceAgentTeam(team.teamId, { dataDir, ...teamOptions });
    } else {
      const approved = intent.type === "approve_decision";
      await decideTeamApproval(team.teamId, intent.payload.taskId, approved ? "approved" : "rejected", intent.payload.note, { dataDir });
      team = await advanceAgentTeam(team.teamId, { dataDir, ...teamOptions });
    }
    const reason = {
      cancel_team: "TEAM_CANCELLED", cancel_agent: "AGENT_CANCELLED", answer_question: "QUESTION_ANSWERED",
      approve_decision: "DECISION_APPROVED", reject_decision: "DECISION_REJECTED",
    }[intent.type];
    return receipt(intent, "applied", reason, completedAt, team);
  } catch {
    projection = await readAgentTeamProjection({ dataDir });
    team = team?.teamId ? projection?.items.find((item) => item.teamId === team.teamId) : null;
    return receipt(intent, "rejected", intent.type === "start_team" ? "TEAM_CONFIGURATION_INVALID" : "TEAM_ACTION_UNAVAILABLE", completedAt, team);
  }
}
