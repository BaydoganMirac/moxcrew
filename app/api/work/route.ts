import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";
import { isAbsolute } from "node:path";
import { supportsCodexControl } from "@/lib/providers/codex.mjs";
import { readProjection } from "@/lib/runtime/observation-store.mjs";
import { readLocalForm } from "@/lib/runtime/local-form.mjs";
import {
  applyWorkCommand,
  applyWorkControlCommand,
} from "@/lib/runtime/work-queue.mjs";
import { resolveRegisteredWorkspace } from "@/lib/runtime/workspace-registry.mjs";

const SHAPES = {
  enqueue: ["action", "commandId", "createdAt", "expiresAt", "title", "instruction", "workspaceId", "workingDirectory"],
  remove: ["action", "commandId", "createdAt", "expiresAt", "workItemId", "expectedItemRevision"],
  start: ["action", "commandId", "createdAt", "expiresAt", "workItemId", "expectedItemRevision"],
  cancel: ["action", "commandId", "createdAt", "expiresAt", "runId", "expectedRunRevision"],
};
const COMMAND_ID = /^cmd:[0-9a-f-]{36}$/;
const INPUT_ERRORS = new Set([
  "WORK_TITLE_REQUIRED",
  "WORK_INSTRUCTION_REQUIRED",
  "WORKSPACE_SELECTION_REQUIRED",
  "WORKSPACE_SELECTION_CONFLICT",
  "WORKSPACE_NOT_REGISTERED",
  "WORKSPACE_PATH_NOT_ABSOLUTE",
]);

function inputError(code: string) {
  return Object.assign(new TypeError(code), { code });
}

function requiredText(value: string, code: string) {
  const normalized = value.trim();
  if (!normalized) throw inputError(code);
  return normalized;
}

function workingDirectory(value: string) {
  const normalized = value.trim();
  if (!isAbsolute(normalized)) throw inputError("WORKSPACE_PATH_NOT_ABSOLUTE");
  return normalized;
}

async function selectedWorkingDirectory(workspaceId: string, manualPath: string) {
  const selected = workspaceId.trim();
  const manual = manualPath.trim();
  if (!selected && !manual) throw inputError("WORKSPACE_SELECTION_REQUIRED");
  if (selected && manual) throw inputError("WORKSPACE_SELECTION_CONFLICT");
  if (!selected) return workingDirectory(manual);
  const workspace = await resolveRegisteredWorkspace(selected);
  if (!workspace) throw inputError("WORKSPACE_NOT_REGISTERED");
  return workspace.workingDirectory;
}

function number(value: string) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 1) throw new TypeError("invalid revision");
  return parsed;
}

function base(form: Record<string, string>, type: string) {
  if (!COMMAND_ID.test(form.commandId)) throw new TypeError("invalid command id");
  return {
    schemaVersion: 1,
    intentId: form.commandId,
    idempotencyKey: form.commandId,
    type,
    actorId: "local-owner",
    createdAt: form.createdAt,
    expiresAt: form.expiresAt,
  };
}

async function codexControlAvailable() {
  const projection = await readProjection();
  return projection?.observations.some(supportsCodexControl) ?? false;
}

function redirect(request: NextRequest, code: string) {
  const target = new URL("/", request.headers.get("origin")!);
  target.searchParams.set("work", code);
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
    let receipt: { reasonCode: string };
    if (form.action === "enqueue") {
      const title = requiredText(form.title, "WORK_TITLE_REQUIRED");
      const instruction = requiredText(form.instruction, "WORK_INSTRUCTION_REQUIRED");
      receipt = await applyWorkCommand({
        ...base(form, "enqueue_work"),
        payload: {
          workItem: {
            schemaVersion: 1,
            workItemId: `work:${form.commandId.slice(4)}`,
            title,
            instruction,
            providerId: "codex",
            workingDirectory: await selectedWorkingDirectory(form.workspaceId, form.workingDirectory),
            status: "queued",
            createdAt: form.createdAt,
            revision: 1,
          },
        },
      });
    } else if (form.action === "remove") {
      receipt = await applyWorkCommand({
        ...base(form, "remove_queued_work"),
        payload: {
          workItemId: form.workItemId,
          expectedItemRevision: number(form.expectedItemRevision),
        },
      });
    } else if (form.action === "start") {
      receipt = await applyWorkControlCommand({
        ...base(form, "start_work"),
        payload: {
          workItemId: form.workItemId,
          expectedItemRevision: number(form.expectedItemRevision),
        },
      }, { providerAvailable: await codexControlAvailable() });
    } else {
      receipt = await applyWorkControlCommand({
        ...base(form, "cancel_run"),
        payload: {
          runId: form.runId,
          expectedRunRevision: number(form.expectedRunRevision),
        },
      }, { providerAvailable: await codexControlAvailable() });
    }
    return redirect(request, receipt.reasonCode);
  } catch (error) {
    if ((error as { outcomeUnknown?: boolean }).outcomeUnknown) {
      return redirect(request, "OUTCOME_PENDING");
    }
    const code = typeof (error as { code?: unknown }).code === "string"
      ? (error as { code: string }).code
      : "INVALID_COMMAND";
    return redirect(request, INPUT_ERRORS.has(code) ? code : "INVALID_COMMAND");
  }
}
