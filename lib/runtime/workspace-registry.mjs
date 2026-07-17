import { randomUUID } from "node:crypto";
import { basename } from "node:path";
import {
  projectWorkspaceEvents,
  validateWorkspaceCommandIntent,
} from "../domain/workspace.mjs";
import { validateCodexWorkspace } from "../providers/codex-control.mjs";
import {
  commitEventTransaction,
  readWorkspaceProjection,
} from "./observation-store.mjs";

export { readWorkspaceProjection };

export class WorkspaceIdempotencyConflictError extends Error {
  constructor() {
    super("Idempotency key was already used by a different workspace command");
    this.code = "IDEMPOTENCY_KEY_REUSED";
  }
}

function event(type, recordedAt, payload) {
  return { id: randomUUID(), schemaVersion: 1, type, recordedAt, payload };
}

function nowValue(now) {
  const value = now();
  if (typeof value !== "string" || new Date(value).toISOString() !== value) {
    throw new TypeError("now must return an ISO timestamp");
  }
  return value;
}

function emptyProjection() {
  return { schemaVersion: 1, revision: 0, items: [], receipts: [] };
}

function existingCommand(events, intent) {
  let requested;
  let receipt;
  for (const item of events) {
    if (item.type === "workspace.command.requested") {
      const candidate = item.payload.intent;
      if (candidate.intentId === intent.intentId || candidate.idempotencyKey === intent.idempotencyKey) {
        if (requested && requested.intentId !== candidate.intentId) throw new WorkspaceIdempotencyConflictError();
        requested = candidate;
      }
    } else if (
      item.type === "workspace.command.receipted" &&
      item.payload.receipt.intentId === intent.intentId
    ) {
      receipt = item.payload.receipt;
    }
  }
  if (requested && JSON.stringify(requested) !== JSON.stringify(intent)) {
    throw new WorkspaceIdempotencyConflictError();
  }
  return { requested, receipt };
}

function receipt(intent, outcome, reasonCode, completedAt, revision, workspaceId) {
  return {
    schemaVersion: 1,
    intentId: intent.intentId,
    idempotencyKey: intent.idempotencyKey,
    commandType: intent.type,
    outcome,
    reasonCode,
    completedAt,
    workspaceProjectionRevision: revision,
    workspaceId,
  };
}

export function applyWorkspaceCommand(value, { dataDir, now = () => new Date().toISOString() } = {}) {
  const intent = validateWorkspaceCommandIntent(value);
  return commitEventTransaction((events) => {
    const existing = existingCommand(events, intent);
    if (existing.receipt) return { additions: [], result: () => existing.receipt };
    const completedAt = nowValue(now);
    const projection = projectWorkspaceEvents(events) ?? emptyProjection();
    const additions = existing.requested
      ? []
      : [event("workspace.command.requested", completedAt, { intent })];
    const existingFact = events.find((candidate) =>
      ["workspace.registered", "workspace.removed"].includes(candidate.type) &&
      candidate.payload.intentId === intent.intentId);
    let outcome = "applied";
    let reasonCode;
    let targetId;

    if (intent.type === "register_workspace") {
      const requested = intent.payload.workspace;
      const duplicate = projection.items.find(
        (item) => item.workingDirectory === requested.workingDirectory,
      );
      targetId = requested.workspaceId;
      if (existingFact?.type === "workspace.registered") {
        reasonCode = "WORKSPACE_REGISTERED";
      } else if (intent.expiresAt <= completedAt) {
        outcome = "expired";
        reasonCode = "COMMAND_EXPIRED";
      } else if (duplicate) {
        targetId = duplicate.workspaceId;
        outcome = "rejected";
        reasonCode = "WORKSPACE_ALREADY_REGISTERED";
      } else {
        additions.push(event("workspace.registered", completedAt, {
          intentId: intent.intentId,
          workspace: requested,
        }));
        projection.revision += 1;
        reasonCode = "WORKSPACE_REGISTERED";
      }
    } else {
      const { workspaceId, expectedWorkspaceRevision } = intent.payload;
      const workspace = projection.items.find((item) => item.workspaceId === workspaceId);
      targetId = workspaceId;
      if (existingFact?.type === "workspace.removed") {
        reasonCode = "WORKSPACE_REMOVED";
      } else if (intent.expiresAt <= completedAt) {
        outcome = "expired";
        reasonCode = "COMMAND_EXPIRED";
      } else if (!workspace) {
        outcome = "rejected";
        reasonCode = "WORKSPACE_NOT_FOUND";
      } else if (workspace.revision !== expectedWorkspaceRevision) {
        outcome = "rejected";
        reasonCode = "STALE_WORKSPACE_REVISION";
      } else {
        additions.push(event("workspace.removed", completedAt, {
          intentId: intent.intentId,
          workspaceId,
          expectedWorkspaceRevision,
        }));
        projection.revision += 1;
        reasonCode = "WORKSPACE_REMOVED";
      }
    }

    const terminal = receipt(
      intent,
      outcome,
      reasonCode,
      completedAt,
      projection.revision,
      targetId,
    );
    additions.push(event("workspace.command.receipted", completedAt, { receipt: terminal }));
    return { additions, result: () => terminal };
  }, { dataDir });
}

function commandBase(now, commandId) {
  return {
    schemaVersion: 1,
    intentId: commandId,
    idempotencyKey: commandId,
    actorId: "local-owner",
    createdAt: now,
    expiresAt: new Date(new Date(now).valueOf() + 5 * 60_000).toISOString(),
  };
}

export async function registerWorkspace(workingDirectory, {
  dataDir,
  now = () => new Date().toISOString(),
  commandId = `cmd:${randomUUID()}`,
  workspaceId = `workspace:${randomUUID()}`,
} = {}) {
  const canonical = await validateCodexWorkspace(workingDirectory);
  const createdAt = nowValue(now);
  const displayName = basename(canonical);
  return applyWorkspaceCommand({
    ...commandBase(createdAt, commandId),
    type: "register_workspace",
    payload: {
      workspace: {
        schemaVersion: 1,
        workspaceId,
        displayName,
        workingDirectory: canonical,
        createdAt,
        revision: 1,
      },
    },
  }, { dataDir, now: () => createdAt });
}

export async function removeWorkspace(workspaceId, {
  dataDir,
  now = () => new Date().toISOString(),
  commandId = `cmd:${randomUUID()}`,
} = {}) {
  const projection = await readWorkspaceProjection({ dataDir }) ?? emptyProjection();
  const workspace = projection.items.find((item) => item.workspaceId === workspaceId);
  const createdAt = nowValue(now);
  return applyWorkspaceCommand({
    ...commandBase(createdAt, commandId),
    type: "remove_workspace",
    payload: {
      workspaceId,
      expectedWorkspaceRevision: workspace?.revision ?? 1,
    },
  }, { dataDir, now: () => createdAt });
}

export async function resolveRegisteredWorkspace(workspaceId, { dataDir } = {}) {
  const projection = await readWorkspaceProjection({ dataDir });
  return projection?.items.find((item) => item.workspaceId === workspaceId) ?? null;
}
