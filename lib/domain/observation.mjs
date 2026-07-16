const PROVIDERS = Object.freeze({
  codex: Object.freeze({
    displayName: "Codex",
    errors: Object.freeze({
      CODEX_APP_SERVER_EXITED: "Codex observation stopped unexpectedly.",
      CODEX_APP_SERVER_START_FAILED: "Codex observation could not start.",
      CODEX_APP_SERVER_TIMEOUT: "Codex observation timed out.",
      CODEX_NOT_FOUND: "Codex CLI is not installed or is not on PATH.",
      CODEX_PROBE_FAILED: "Codex CLI could not be checked.",
      CODEX_PROBE_TIMEOUT: "Codex CLI version check timed out.",
      CODEX_PROTOCOL_INVALID_JSON: "Codex returned an unsupported response.",
      CODEX_PROTOCOL_MALFORMED: "Codex returned an unsupported response.",
      CODEX_PROTOCOL_METHOD_ERROR: "Codex rejected the observation request.",
      CODEX_SYSTEM_ERROR: "Codex reported an observation error.",
      CODEX_VERSION_MALFORMED: "Codex CLI returned an unsupported version.",
    }),
  }),
  claude: Object.freeze({
    displayName: "Claude Code",
    errors: Object.freeze({
      CLAUDE_NOT_FOUND: "Claude Code CLI is not installed or is not on PATH.",
      CLAUDE_PROBE_FAILED: "Claude Code CLI could not be checked.",
      CLAUDE_PROBE_TIMEOUT: "Claude Code CLI version check timed out.",
      CLAUDE_SNAPSHOT_FAILED: "Claude Code Agent View could not be checked.",
      CLAUDE_SNAPSHOT_MALFORMED: "Claude Code Agent View returned an unsupported response.",
      CLAUDE_SNAPSHOT_TIMEOUT: "Claude Code Agent View timed out.",
      CLAUDE_SNAPSHOT_UNSUPPORTED: "Claude Code Agent View returned an unsupported session shape.",
      CLAUDE_SURFACE_UNSUPPORTED: "This Claude Code version lacks the supported Agent View surface.",
      CLAUDE_VERSION_MALFORMED: "Claude Code CLI returned an unsupported version.",
    }),
  }),
  gemini: Object.freeze({
    displayName: "Gemini CLI",
    errors: Object.freeze({
      GEMINI_HOOK_SETUP_REQUIRED: "Gemini CLI hook setup is required.",
      GEMINI_NOT_FOUND: "Gemini CLI is not installed or is not on PATH.",
      GEMINI_PROBE_FAILED: "Gemini CLI could not be checked.",
      GEMINI_PROBE_TIMEOUT: "Gemini CLI version check timed out.",
      GEMINI_VERSION_MALFORMED: "Gemini CLI returned an unsupported version.",
    }),
  }),
});

const STATES = new Set(["available", "degraded", "unavailable"]);
const SESSION_STATES = new Set([
  "completed",
  "failed",
  "interrupted",
  "running",
  "unknown",
]);
const OPAQUE_ID = /^[A-Za-z0-9._:-]{1,256}$/;
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

function record(value, name) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${name} must be an object`);
  }
  return value;
}

function iso(value, name) {
  if (typeof value !== "string" || new Date(value).toISOString() !== value) {
    throw new TypeError(`${name} must be an ISO timestamp`);
  }
  return value;
}

function exactKeys(value, allowed, name) {
  for (const key of Object.keys(value)) {
    if (!allowed.includes(key)) throw new TypeError(`${name} has an unknown field`);
  }
}

export function observationProvider(providerId) {
  if (!Object.hasOwn(PROVIDERS, providerId)) throw new TypeError("Invalid provider identity");
  return { id: providerId, displayName: PROVIDERS[providerId].displayName };
}

export function safeObservationError(providerId, code) {
  const errors = PROVIDERS[providerId]?.errors;
  if (!errors || !Object.hasOwn(errors, code)) {
    throw new TypeError("Unknown observation error code");
  }
  return { code, message: errors[code] };
}

export function normalizeObservation(value) {
  const input = record(value, "observation");
  const provider = record(input.provider, "provider");
  const identity = observationProvider(provider.id);
  if (provider.displayName !== identity.displayName) throw new TypeError("Invalid provider identity");
  if (!STATES.has(provider.state)) throw new TypeError("Invalid provider state");
  if (provider.version !== null && !VERSION.test(provider.version)) {
    throw new TypeError("Invalid provider version");
  }

  const capabilities = record(provider.capabilities, "capabilities");
  if (
    typeof capabilities.recentObservation !== "boolean" ||
    typeof capabilities.explicitLiveStatus !== "boolean"
  ) {
    throw new TypeError("Invalid provider capabilities");
  }

  const error = provider.error ? safeObservationError(provider.id, provider.error.code) : undefined;
  if ((provider.state === "available") === Boolean(error)) {
    throw new TypeError("Provider state and error disagree");
  }

  if (!Array.isArray(input.sessions) || input.sessions.length > 20) {
    throw new TypeError("Invalid session list");
  }

  const seen = new Set();
  const sessions = input.sessions.map((candidate) => {
    const session = record(candidate, "session");
    if (!OPAQUE_ID.test(session.providerSessionId) || seen.has(session.providerSessionId)) {
      throw new TypeError("Invalid provider session id");
    }
    if (!SESSION_STATES.has(session.status)) throw new TypeError("Invalid session state");
    seen.add(session.providerSessionId);

    const normalized = {
      providerSessionId: session.providerSessionId,
      status: session.status,
      createdAt: session.createdAt === null ? null : iso(session.createdAt, "createdAt"),
      lastObservedAt: iso(session.lastObservedAt, "lastObservedAt"),
    };
    if (session.terminalAt !== undefined) {
      normalized.terminalAt = iso(session.terminalAt, "terminalAt");
    }
    return normalized;
  });

  if (provider.state === "unavailable" && sessions.length) {
    throw new TypeError("Unavailable provider cannot contain sessions");
  }

  return {
    schemaVersion: 1,
    provider: {
      ...identity,
      state: provider.state,
      version: provider.version,
      capabilities: {
        recentObservation: capabilities.recentObservation,
        explicitLiveStatus: capabilities.explicitLiveStatus,
      },
      ...(error ? { error } : {}),
    },
    observedAt: iso(input.observedAt, "observedAt"),
    sessions,
  };
}

export function validateProjection(value) {
  const input = record(value, "projection");
  if (input.schemaVersion === 1) {
    exactKeys(input, ["schemaVersion", "provider", "observedAt", "sessions"], "projection");
    const observation = validateStoredObservation(input);
    if (observation.provider.id !== "codex") throw new TypeError("Invalid legacy projection");
    return { schemaVersion: 2, observations: [observation] };
  }
  exactKeys(input, ["schemaVersion", "observations"], "projection");
  if (input.schemaVersion !== 2 || !Array.isArray(input.observations)) {
    throw new TypeError("Unsupported projection version");
  }
  const seen = new Set();
  const observations = input.observations.map((observation) => {
    const normalized = validateStoredObservation(observation);
    if (seen.has(normalized.provider.id)) throw new TypeError("Duplicate provider observation");
    seen.add(normalized.provider.id);
    return normalized;
  });
  const order = Object.keys(PROVIDERS);
  observations.sort((left, right) => order.indexOf(left.provider.id) - order.indexOf(right.provider.id));
  return { schemaVersion: 2, observations };
}

function validateStoredObservation(value) {
  const input = record(value, "observation");
  exactKeys(input, ["schemaVersion", "provider", "observedAt", "sessions"], "observation");
  if (input.schemaVersion !== 1) throw new TypeError("Unsupported observation version");

  const provider = record(input.provider, "provider");
  exactKeys(
    provider,
    ["id", "displayName", "state", "version", "capabilities", "error"],
    "provider",
  );
  exactKeys(
    record(provider.capabilities, "capabilities"),
    ["recentObservation", "explicitLiveStatus"],
    "capabilities",
  );
  if (provider.error) {
    exactKeys(record(provider.error, "error"), ["code", "message"], "error");
    if (safeObservationError(provider.id, provider.error.code).message !== provider.error.message) {
      throw new TypeError("Unsafe observation error message");
    }
  }
  if (!Array.isArray(input.sessions)) throw new TypeError("Invalid session list");
  for (const session of input.sessions) {
    exactKeys(
      record(session, "session"),
      ["providerSessionId", "status", "createdAt", "lastObservedAt", "terminalAt"],
      "session",
    );
  }
  return normalizeObservation(input);
}
