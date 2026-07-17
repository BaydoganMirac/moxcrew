import { isAbsolute } from "node:path";

const UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";
const COMMAND_ID = new RegExp(`^cmd:${UUID}$`, "i");
const WORKSPACE_ID = new RegExp(`^workspace:${UUID}$`, "i");
const COMMAND_TYPES = new Set(["register_workspace", "remove_workspace"]);
const OUTCOMES = new Set(["applied", "rejected", "expired"]);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`invalid ${label}`);
  return value;
}

function exact(value, keys, label) {
  const actual = Object.keys(record(value, label));
  if (actual.length !== keys.length || actual.some((key) => !keys.includes(key))) {
    throw new TypeError(`invalid ${label}`);
  }
}

function iso(value, label) {
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError(`invalid ${label}`);
  return value;
}

function revision(value, label, minimum = 0) {
  if (!Number.isSafeInteger(value) || value < minimum) throw new TypeError(`invalid ${label}`);
  return value;
}

function commandId(value, label) {
  if (typeof value !== "string" || !COMMAND_ID.test(value)) throw new TypeError(`invalid ${label}`);
  return value;
}

function workspaceId(value) {
  if (typeof value !== "string" || !WORKSPACE_ID.test(value)) throw new TypeError("invalid workspace id");
  return value;
}

export function validateWorkspace(value) {
  const input = record(value, "workspace");
  exact(input, [
    "schemaVersion",
    "workspaceId",
    "displayName",
    "workingDirectory",
    "createdAt",
    "revision",
  ], "workspace");
  if (
    input.schemaVersion !== 1 ||
    typeof input.displayName !== "string" ||
    input.displayName !== input.displayName.trim() ||
    input.displayName.length < 1 ||
    input.displayName.length > 80 ||
    typeof input.workingDirectory !== "string" ||
    input.workingDirectory !== input.workingDirectory.trim() ||
    input.workingDirectory.length > 4096 ||
    !isAbsolute(input.workingDirectory)
  ) {
    throw new TypeError("invalid workspace");
  }
  return {
    schemaVersion: 1,
    workspaceId: workspaceId(input.workspaceId),
    displayName: input.displayName,
    workingDirectory: input.workingDirectory,
    createdAt: iso(input.createdAt, "workspace creation time"),
    revision: revision(input.revision, "workspace revision", 1),
  };
}

export function validateWorkspaceCommandIntent(value) {
  const input = record(value, "workspace command intent");
  exact(input, [
    "schemaVersion",
    "intentId",
    "idempotencyKey",
    "type",
    "actorId",
    "createdAt",
    "expiresAt",
    "payload",
  ], "workspace command intent");
  if (
    input.schemaVersion !== 1 ||
    input.actorId !== "local-owner" ||
    !COMMAND_TYPES.has(input.type)
  ) {
    throw new TypeError("invalid workspace command intent");
  }
  const intentId = commandId(input.intentId, "intent id");
  const idempotencyKey = commandId(input.idempotencyKey, "idempotency key");
  const createdAt = iso(input.createdAt, "command creation time");
  const expiresAt = iso(input.expiresAt, "command expiry time");
  if (expiresAt <= createdAt) throw new TypeError("invalid command expiry");
  const payload = record(input.payload, "workspace command payload");
  if (input.type === "register_workspace") {
    exact(payload, ["workspace"], "register workspace payload");
    return {
      ...input,
      intentId,
      idempotencyKey,
      createdAt,
      expiresAt,
      payload: { workspace: validateWorkspace(payload.workspace) },
    };
  }
  exact(payload, ["workspaceId", "expectedWorkspaceRevision"], "remove workspace payload");
  return {
    ...input,
    intentId,
    idempotencyKey,
    createdAt,
    expiresAt,
    payload: {
      workspaceId: workspaceId(payload.workspaceId),
      expectedWorkspaceRevision: revision(payload.expectedWorkspaceRevision, "expected workspace revision", 1),
    },
  };
}

export function validateWorkspaceCommandReceipt(value) {
  const input = record(value, "workspace command receipt");
  exact(input, [
    "schemaVersion",
    "intentId",
    "idempotencyKey",
    "commandType",
    "outcome",
    "reasonCode",
    "completedAt",
    "workspaceProjectionRevision",
    "workspaceId",
  ], "workspace command receipt");
  if (
    input.schemaVersion !== 1 ||
    !COMMAND_TYPES.has(input.commandType) ||
    !OUTCOMES.has(input.outcome) ||
    typeof input.reasonCode !== "string" ||
    !/^[A-Z][A-Z0-9_]{1,63}$/.test(input.reasonCode)
  ) {
    throw new TypeError("invalid workspace command receipt");
  }
  return {
    ...input,
    intentId: commandId(input.intentId, "receipt intent id"),
    idempotencyKey: commandId(input.idempotencyKey, "receipt idempotency key"),
    completedAt: iso(input.completedAt, "receipt completion time"),
    workspaceProjectionRevision: revision(input.workspaceProjectionRevision, "workspace projection revision"),
    workspaceId: workspaceId(input.workspaceId),
  };
}

export function validateWorkspaceEvent(value) {
  const input = record(value, "workspace event");
  const payload = record(input.payload, "workspace event payload");
  if (input.type === "workspace.command.requested") {
    exact(payload, ["intent"], "workspace command requested payload");
    return { ...input, payload: { intent: validateWorkspaceCommandIntent(payload.intent) } };
  }
  if (input.type === "workspace.registered") {
    exact(payload, ["intentId", "workspace"], "workspace registered payload");
    return {
      ...input,
      payload: {
        intentId: commandId(payload.intentId, "workspace event intent id"),
        workspace: validateWorkspace(payload.workspace),
      },
    };
  }
  if (input.type === "workspace.removed") {
    exact(payload, ["intentId", "workspaceId", "expectedWorkspaceRevision"], "workspace removed payload");
    return {
      ...input,
      payload: {
        intentId: commandId(payload.intentId, "workspace event intent id"),
        workspaceId: workspaceId(payload.workspaceId),
        expectedWorkspaceRevision: revision(payload.expectedWorkspaceRevision, "expected workspace revision", 1),
      },
    };
  }
  if (input.type === "workspace.command.receipted") {
    exact(payload, ["receipt"], "workspace command receipted payload");
    return { ...input, payload: { receipt: validateWorkspaceCommandReceipt(payload.receipt) } };
  }
  throw new TypeError("unsupported workspace event");
}

export function validateWorkspaceProjection(value) {
  const input = record(value, "workspace projection");
  exact(input, ["schemaVersion", "revision", "items", "receipts"], "workspace projection");
  if (input.schemaVersion !== 1 || !Array.isArray(input.items) || !Array.isArray(input.receipts)) {
    throw new TypeError("unsupported workspace projection");
  }
  const items = input.items.map(validateWorkspace);
  const receipts = input.receipts.map(validateWorkspaceCommandReceipt);
  if (
    new Set(items.map((item) => item.workspaceId)).size !== items.length ||
    new Set(items.map((item) => item.workingDirectory)).size !== items.length ||
    new Set(receipts.map((item) => item.intentId)).size !== receipts.length
  ) {
    throw new TypeError("duplicate workspace projection entry");
  }
  return {
    schemaVersion: 1,
    revision: revision(input.revision, "workspace projection revision"),
    items,
    receipts,
  };
}

export function projectWorkspaceEvents(events) {
  const intents = new Map();
  const keys = new Set();
  const items = new Map();
  const pathIds = new Map();
  const facts = new Map();
  const snapshots = new Map();
  const receipts = [];
  const receipted = new Set();
  let revisionValue = 0;
  let found = false;

  for (const candidate of events) {
    if (!candidate.type.startsWith("workspace.")) continue;
    found = true;
    const event = validateWorkspaceEvent(candidate);
    if (event.type === "workspace.command.requested") {
      const { intent } = event.payload;
      if (intents.has(intent.intentId) || keys.has(intent.idempotencyKey)) throw new TypeError("duplicate workspace command");
      intents.set(intent.intentId, intent);
      keys.add(intent.idempotencyKey);
      if (intent.type === "remove_workspace") {
        snapshots.set(intent.intentId, items.get(intent.payload.workspaceId) ?? null);
      }
    } else if (event.type === "workspace.registered") {
      const intent = intents.get(event.payload.intentId);
      const workspace = event.payload.workspace;
      if (
        !intent ||
        intent.type !== "register_workspace" ||
        facts.has(intent.intentId) ||
        JSON.stringify(intent.payload.workspace) !== JSON.stringify(workspace) ||
        items.has(workspace.workspaceId) ||
        pathIds.has(workspace.workingDirectory)
      ) {
        throw new TypeError("invalid workspace registration event");
      }
      items.set(workspace.workspaceId, workspace);
      pathIds.set(workspace.workingDirectory, workspace.workspaceId);
      facts.set(intent.intentId, event.type);
      revisionValue += 1;
    } else if (event.type === "workspace.removed") {
      const intent = intents.get(event.payload.intentId);
      const workspace = items.get(event.payload.workspaceId);
      if (
        !intent ||
        intent.type !== "remove_workspace" ||
        facts.has(intent.intentId) ||
        intent.payload.workspaceId !== event.payload.workspaceId ||
        intent.payload.expectedWorkspaceRevision !== event.payload.expectedWorkspaceRevision ||
        !workspace ||
        workspace.revision !== event.payload.expectedWorkspaceRevision
      ) {
        throw new TypeError("invalid workspace removal event");
      }
      items.delete(workspace.workspaceId);
      pathIds.delete(workspace.workingDirectory);
      facts.set(intent.intentId, event.type);
      revisionValue += 1;
    } else {
      const { receipt } = event.payload;
      const intent = intents.get(receipt.intentId);
      const fact = facts.get(receipt.intentId);
      let outcomeIsValid = false;
      if (intent?.type === "register_workspace") {
        const requested = intent.payload.workspace;
        const existing = [...items.values()].find((item) => item.workingDirectory === requested.workingDirectory);
        outcomeIsValid =
          (receipt.outcome === "applied" && fact === "workspace.registered" && receipt.reasonCode === "WORKSPACE_REGISTERED" && receipt.workspaceId === requested.workspaceId) ||
          (receipt.outcome === "rejected" && !fact && receipt.reasonCode === "WORKSPACE_ALREADY_REGISTERED" && receipt.workspaceId === existing?.workspaceId);
      } else if (intent?.type === "remove_workspace") {
        const snapshot = snapshots.get(intent.intentId);
        outcomeIsValid =
          (receipt.outcome === "applied" && fact === "workspace.removed" && receipt.reasonCode === "WORKSPACE_REMOVED" && receipt.workspaceId === intent.payload.workspaceId) ||
          (receipt.outcome === "rejected" && !fact && receipt.reasonCode === "WORKSPACE_NOT_FOUND" && !snapshot && receipt.workspaceId === intent.payload.workspaceId) ||
          (receipt.outcome === "rejected" && !fact && receipt.reasonCode === "STALE_WORKSPACE_REVISION" && snapshot?.revision !== intent.payload.expectedWorkspaceRevision && receipt.workspaceId === intent.payload.workspaceId);
      }
      outcomeIsValid = outcomeIsValid || (
        receipt.outcome === "expired" &&
        !fact &&
        receipt.reasonCode === "COMMAND_EXPIRED" &&
        receipt.completedAt >= intent?.expiresAt &&
        receipt.workspaceId === (intent?.type === "register_workspace"
          ? intent.payload.workspace.workspaceId
          : intent?.payload.workspaceId)
      );
      if (
        !intent ||
        intent.idempotencyKey !== receipt.idempotencyKey ||
        intent.type !== receipt.commandType ||
        receipted.has(receipt.intentId) ||
        receipt.workspaceProjectionRevision !== revisionValue ||
        !outcomeIsValid
      ) {
        throw new TypeError("invalid workspace command receipt event");
      }
      receipts.push(receipt);
      receipted.add(receipt.intentId);
    }
  }

  if (!found) return null;
  return validateWorkspaceProjection({
    schemaVersion: 1,
    revision: revisionValue,
    items: [...items.values()].sort(
      (left, right) => left.displayName.localeCompare(right.displayName) || left.workspaceId.localeCompare(right.workspaceId),
    ),
    receipts,
  });
}
