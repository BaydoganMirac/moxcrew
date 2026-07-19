import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";
import { readLocalForm } from "@/lib/runtime/local-form.mjs";
import {
  advanceAgentTeam,
  answerTeamQuestion,
  cancelAgentTeam,
  cancelTeamAgent,
  createAgentTeam,
  decideTeamApproval,
  startAgentTeam,
} from "@/lib/runtime/team-orchestrator.mjs";

const SHAPES = {
  create: ["action", "name", "goal", "workspaceId", "templateId", "orchestratorPackId", "workerPack1", "workerPack2", "workerPack3", "workerPack4", "concurrency", "retryLimit", "timeoutMinutes", "failurePolicy"],
  start: ["action", "teamId"],
  advance: ["action", "teamId"],
  cancel: ["action", "teamId"],
  cancel_agent: ["action", "teamId", "agentId"],
  answer: ["action", "teamId", "questionId", "answer"],
  approve: ["action", "teamId", "taskId", "note"],
  reject: ["action", "teamId", "taskId", "note"],
};

function integer(value: string, minimum: number, maximum: number) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) throw new TypeError("invalid team limit");
  return parsed;
}

function redirect(request: NextRequest, code: string) {
  const target = new URL("/", request.headers.get("origin")!);
  target.searchParams.set("team", code);
  return NextResponse.redirect(target, 303);
}

export async function POST(request: NextRequest) {
  let form: Record<string, string>;
  try {
    form = await readLocalForm(request, SHAPES) as Record<string, string>;
  } catch {
    return new NextResponse("Forbidden", { status: 403 });
  }
  try {
    if (form.action === "create") {
      await createAgentTeam({
        name: form.name,
        goal: form.goal,
        workspaceId: form.workspaceId,
        templateId: form.templateId,
        orchestratorPackId: form.orchestratorPackId,
        workerPackIds: [form.workerPack1, form.workerPack2, form.workerPack3, form.workerPack4].filter(Boolean),
        settings: {
          concurrency: integer(form.concurrency, 1, 4),
          retryLimit: integer(form.retryLimit, 0, 3),
          timeoutMinutes: integer(form.timeoutMinutes, 5, 480),
          failurePolicy: form.failurePolicy,
        },
      });
      return redirect(request, "TEAM_CREATED");
    }
    if (form.action === "start") {
      await startAgentTeam(form.teamId);
      return redirect(request, "TEAM_STARTED");
    }
    if (form.action === "advance") {
      await advanceAgentTeam(form.teamId);
      return redirect(request, "TEAM_ADVANCED");
    }
    if (form.action === "cancel") {
      await cancelAgentTeam(form.teamId);
      return redirect(request, "TEAM_CANCELLED");
    }
    if (form.action === "cancel_agent") {
      await cancelTeamAgent(form.teamId, form.agentId);
      return redirect(request, "AGENT_CANCELLED");
    }
    if (form.action === "answer") {
      await answerTeamQuestion(form.teamId, form.questionId, form.answer);
      await advanceAgentTeam(form.teamId);
      return redirect(request, "QUESTION_ANSWERED");
    }
    const decision = form.action === "approve" ? "approved" : "rejected";
    await decideTeamApproval(form.teamId, form.taskId, decision, form.note);
    await advanceAgentTeam(form.teamId);
    return redirect(request, decision === "approved" ? "TASK_APPROVED" : "TASK_REJECTED");
  } catch {
    return redirect(request, "TEAM_INVALID_ACTION");
  }
}
