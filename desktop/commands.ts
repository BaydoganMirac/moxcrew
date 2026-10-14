export type FormNamespace = "observe" | "workspace" | "work" | "team" | "pack" | "cloud";
export type HostCommand = { action: string; payload: Record<string, unknown> };

const ACTIONS: Record<FormNamespace, Record<string, string>> = {
  observe: { "": "observe" },
  workspace: { register: "workspace_register", remove: "workspace_remove" },
  work: { enqueue: "work_enqueue", start: "work_start", retry: "work_retry", cancel: "work_cancel", remove: "work_remove" },
  team: { create: "team_create", start: "team_start", advance: "team_advance", cancel: "team_cancel", cancel_agent: "team_cancel_agent", approve: "team_approve", reject: "team_approve", answer: "team_answer" },
  pack: { install: "pack_install", remove: "pack_remove" },
  cloud: { pair: "cloud_pair", disconnect: "cloud_unpair", sync: "cloud_sync" },
};

const INTEGER_FIELDS = new Set(["expectedItemRevision", "expectedRunRevision", "expectedWorkspaceRevision", "expectedTeamRevision"]);

function uuid() {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 15) | 64;
  bytes[8] = (bytes[8] & 63) | 128;
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

export function commandFromForm(namespace: FormNamespace, values: Record<string, unknown>, now = new Date(), id = uuid()): HostCommand {
  const formAction = typeof values.action === "string" ? values.action : "";
  if (!Object.prototype.hasOwnProperty.call(ACTIONS[namespace], formAction)) throw new Error("Unsupported action");
  const action = ACTIONS[namespace][formAction];
  if (!action) throw new Error("Unsupported action");
  const payload: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(values)) {
    if (key === "action") continue;
    if (typeof value !== "string") throw new Error("Only text form values are accepted");
    if (INTEGER_FIELDS.has(key)) {
      const integer = Number(value);
      if (!Number.isSafeInteger(integer) || integer < 1) throw new Error("Invalid state revision");
      payload[key] = integer;
    } else payload[key] = value;
  }
  if (action === "team_create") {
    payload.workerPackIds = [1, 2, 3, 4].map((slot) => payload[`workerPack${slot}`]).filter(Boolean);
    for (const slot of [1, 2, 3, 4]) delete payload[`workerPack${slot}`];
    const settings: Record<string, unknown> = { failurePolicy: payload.failurePolicy };
    for (const [key, min, max] of [["concurrency", 1, 4], ["retryLimit", 0, 3], ["timeoutMinutes", 5, 480]] as const) {
      const value = Number(payload[key]);
      if (!Number.isSafeInteger(value) || value < min || value > max) throw new Error("Invalid team limit");
      settings[key] = value;
      delete payload[key];
    }
    delete payload.failurePolicy;
    payload.settings = settings;
  }
  if (action === "team_approve") payload.decision = formAction === "approve" ? "approved" : "rejected";
  if (action === "pack_install") {
    try { payload.manifest = JSON.parse(String(payload.manifest)); } catch { throw new Error("Pack manifest must be valid JSON"); }
    if (!payload.manifest || typeof payload.manifest !== "object" || Array.isArray(payload.manifest)) throw new Error("Pack manifest must be a JSON object");
  }
  payload.commandId = `cmd:${id}`;
  payload.createdAt = now.toISOString();
  payload.expiresAt = new Date(now.valueOf() + 5 * 60_000).toISOString();
  return { action, payload };
}
