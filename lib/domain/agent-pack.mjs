const PACK_ID = /^pack:[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$/;
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const ROLES = new Set([
  "orchestrator", "product", "design", "frontend", "backend", "fullstack",
  "qa", "review", "security", "release", "docs", "research",
]);
const CAPABILITIES = new Set(["work.start", "work.cancel"]);
const PERMISSIONS = new Set(["read_workspace", "write_workspace", "run_checks"]);
const QUALITY_CHECKS = new Set(["tests", "lint", "build", "review", "security"]);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`invalid ${label}`);
  return value;
}

function exact(value, fields, label) {
  const keys = Object.keys(record(value, label));
  if (keys.length !== fields.length || keys.some((key) => !fields.includes(key))) {
    throw new TypeError(`invalid ${label}`);
  }
}

function text(value, label, maximum) {
  if (
    typeof value !== "string" || value !== value.trim() || value.length < 1 ||
    value.length > maximum || /[\0\u0001-\u0008\u000b\u000c\u000e-\u001f\u007f]/.test(value)
  ) throw new TypeError(`invalid ${label}`);
  return value;
}

function uniqueEnum(values, allowed, label, { minimum = 0 } = {}) {
  if (
    !Array.isArray(values) || values.length < minimum || values.length > allowed.size ||
    new Set(values).size !== values.length || values.some((value) => !allowed.has(value))
  ) throw new TypeError(`invalid ${label}`);
  return [...values];
}

export function comparePackVersions(left, right) {
  if (!VERSION.test(left) || !VERSION.test(right)) throw new TypeError("invalid agent pack version");
  const a = left.split(".").map(Number);
  const b = right.split(".").map(Number);
  for (let index = 0; index < 3; index += 1) {
    if (a[index] !== b[index]) return a[index] < b[index] ? -1 : 1;
  }
  return 0;
}

export function validateAgentPack(value) {
  const input = record(value, "agent pack");
  exact(input, [
    "schemaVersion", "id", "version", "name", "role", "description",
    "providerId", "instructions", "requiredCapabilities", "permissions",
    "defaultModel", "limits", "expectedOutput", "qualityChecks", "provenance",
  ], "agent pack");
  if (
    input.schemaVersion !== 1 || !PACK_ID.test(input.id) || !VERSION.test(input.version) ||
    !ROLES.has(input.role) || input.providerId !== "codex"
  ) throw new TypeError("invalid agent pack");
  if (input.defaultModel !== null) text(input.defaultModel, "default model", 120);
  exact(input.limits, ["maxAttempts", "timeoutMinutes"], "agent pack limits");
  if (
    !Number.isSafeInteger(input.limits.maxAttempts) || input.limits.maxAttempts < 1 || input.limits.maxAttempts > 5 ||
    !Number.isSafeInteger(input.limits.timeoutMinutes) || input.limits.timeoutMinutes < 1 || input.limits.timeoutMinutes > 240
  ) throw new TypeError("invalid agent pack limits");
  exact(input.provenance, ["kind", "source"], "agent pack provenance");
  if (!["built-in", "local"].includes(input.provenance.kind)) throw new TypeError("invalid agent pack provenance");

  return {
    schemaVersion: 1,
    id: input.id,
    version: input.version,
    name: text(input.name, "agent pack name", 80),
    role: input.role,
    description: text(input.description, "agent pack description", 280),
    providerId: "codex",
    instructions: text(input.instructions, "agent pack instructions", 20_000),
    requiredCapabilities: uniqueEnum(input.requiredCapabilities, CAPABILITIES, "agent pack capabilities", { minimum: 1 }),
    permissions: uniqueEnum(input.permissions, PERMISSIONS, "agent pack permissions", { minimum: 1 }),
    defaultModel: input.defaultModel,
    limits: { maxAttempts: input.limits.maxAttempts, timeoutMinutes: input.limits.timeoutMinutes },
    expectedOutput: text(input.expectedOutput, "expected output", 500),
    qualityChecks: uniqueEnum(input.qualityChecks, QUALITY_CHECKS, "quality checks"),
    provenance: {
      kind: input.provenance.kind,
      source: text(input.provenance.source, "agent pack provenance source", 120),
    },
  };
}

export function validateAgentPackEvent(value) {
  const input = record(value, "agent pack event");
  const payload = record(input.payload, "agent pack event payload");
  if (input.type === "agent_pack.installed") {
    exact(payload, ["pack"], "agent pack installed payload");
    const pack = validateAgentPack(payload.pack);
    if (pack.provenance.kind !== "local") throw new TypeError("only local packs can be installed");
    return { ...input, payload: { pack } };
  }
  if (input.type === "agent_pack.removed") {
    exact(payload, ["packId", "expectedRevision"], "agent pack removed payload");
    if (!PACK_ID.test(payload.packId) || !Number.isSafeInteger(payload.expectedRevision) || payload.expectedRevision < 1) {
      throw new TypeError("invalid agent pack removal");
    }
    return { ...input, payload: { packId: payload.packId, expectedRevision: payload.expectedRevision } };
  }
  throw new TypeError("unsupported agent pack event");
}

export function validateAgentPackProjection(value) {
  const input = record(value, "agent pack projection");
  exact(input, ["schemaVersion", "revision", "items"], "agent pack projection");
  if (input.schemaVersion !== 1 || !Number.isSafeInteger(input.revision) || input.revision < 0 || !Array.isArray(input.items)) {
    throw new TypeError("invalid agent pack projection");
  }
  const ids = new Set();
  const items = input.items.map((item) => {
    exact(item, ["pack", "revision", "installedAt"], "installed agent pack");
    const pack = validateAgentPack(item.pack);
    if (pack.provenance.kind !== "local" || ids.has(pack.id)) throw new TypeError("invalid installed agent pack");
    ids.add(pack.id);
    if (!Number.isSafeInteger(item.revision) || item.revision < 1) throw new TypeError("invalid installed pack revision");
    if (typeof item.installedAt !== "string" || new Date(item.installedAt).toISOString() !== item.installedAt) {
      throw new TypeError("invalid installed pack time");
    }
    return { pack, revision: item.revision, installedAt: item.installedAt };
  });
  return { schemaVersion: 1, revision: input.revision, items };
}

export function projectAgentPackEvents(events) {
  const items = new Map();
  let revision = 0;
  let found = false;
  for (const candidate of events) {
    if (!candidate.type.startsWith("agent_pack.")) continue;
    found = true;
    const event = validateAgentPackEvent(candidate);
    if (event.type === "agent_pack.installed") {
      const current = items.get(event.payload.pack.id);
      if (current && comparePackVersions(event.payload.pack.version, current.pack.version) <= 0) {
        throw new TypeError("agent pack version must increase");
      }
      items.set(event.payload.pack.id, {
        pack: event.payload.pack,
        revision: (current?.revision ?? 0) + 1,
        installedAt: event.recordedAt,
      });
    } else {
      const current = items.get(event.payload.packId);
      if (!current || current.revision !== event.payload.expectedRevision) throw new TypeError("invalid agent pack removal event");
      items.delete(event.payload.packId);
    }
    revision += 1;
  }
  if (!found) return null;
  return validateAgentPackProjection({
    schemaVersion: 1,
    revision,
    items: [...items.values()].sort((left, right) => left.pack.name.localeCompare(right.pack.name) || left.pack.id.localeCompare(right.pack.id)),
  });
}
