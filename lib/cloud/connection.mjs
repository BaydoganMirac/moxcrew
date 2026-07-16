import { randomUUID } from "node:crypto";
import { chmod, mkdir, open, readFile, rename } from "node:fs/promises";
import { join } from "node:path";
import { resolvePatchfleetDataDirectory } from "../runtime/gemini-inbox.mjs";

const FILE = "cloud.json";
const OPAQUE = /^[A-Za-z0-9._:-]{1,256}$/;
const ERROR = /^[A-Z][A-Z0-9_]{0,63}$/;

function exact(value, fields, name) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`${name} is invalid`);
  const keys = Object.keys(value);
  if (keys.length !== fields.length || !keys.every((key) => fields.includes(key))) {
    throw new TypeError(`${name} fields must match`);
  }
}

function opaque(value, name) {
  if (typeof value !== "string" || !OPAQUE.test(value)) throw new TypeError(`${name} is invalid`);
  return value;
}

function nullableIso(value, name) {
  if (value === null) return null;
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError(`${name} is invalid`);
  return value;
}

export function normalizeCloudUrl(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new TypeError("invalid Cloud URL");
  }
  const loopback = ["localhost", "127.0.0.1", "[::1]"].includes(parsed.hostname);
  if (
    (parsed.protocol !== "https:" && !(parsed.protocol === "http:" && loopback)) ||
    parsed.username || parsed.password || parsed.pathname !== "/" || parsed.search || parsed.hash
  ) {
    throw new TypeError("Cloud URL must be HTTPS or exact loopback HTTP");
  }
  return parsed.origin;
}

export function validateCloudState(value) {
  exact(value, ["schemaVersion", "installationId", "connection"], "Cloud state");
  if (value.schemaVersion !== 1) throw new TypeError("unsupported Cloud state");
  const installationId = opaque(value.installationId, "installationId");
  if (value.connection === null) return { schemaVersion: 1, installationId, connection: null };
  const connection = value.connection;
  exact(connection, [
    "cloudUrl", "hostId", "workspaceId", "credential", "protocolVersion", "cursor",
    "lastAttemptAt", "lastSuccessAt", "lastErrorCode",
  ], "Cloud connection");
  if (connection.protocolVersion !== 1 || !Number.isSafeInteger(connection.cursor) || connection.cursor < 0) {
    throw new TypeError("invalid Cloud connection version or cursor");
  }
  if (connection.lastErrorCode !== null && (typeof connection.lastErrorCode !== "string" || !ERROR.test(connection.lastErrorCode))) {
    throw new TypeError("invalid Cloud error code");
  }
  return {
    schemaVersion: 1,
    installationId,
    connection: {
      cloudUrl: normalizeCloudUrl(connection.cloudUrl),
      hostId: opaque(connection.hostId, "hostId"),
      workspaceId: opaque(connection.workspaceId, "workspaceId"),
      credential: opaque(connection.credential, "credential"),
      protocolVersion: 1,
      cursor: connection.cursor,
      lastAttemptAt: nullableIso(connection.lastAttemptAt, "lastAttemptAt"),
      lastSuccessAt: nullableIso(connection.lastSuccessAt, "lastSuccessAt"),
      lastErrorCode: connection.lastErrorCode,
    },
  };
}

export async function readCloudState({ dataDir } = {}) {
  const directory = resolvePatchfleetDataDirectory(dataDir);
  try {
    return validateCloudState(JSON.parse(await readFile(join(directory, FILE), "utf8")));
  } catch (error) {
    if (error.code === "ENOENT") {
      return { schemaVersion: 1, installationId: `install:${randomUUID()}`, connection: null };
    }
    throw error;
  }
}

export async function writeCloudState(value, { dataDir } = {}) {
  const state = validateCloudState(value);
  const directory = resolvePatchfleetDataDirectory(dataDir);
  const createdDirectory = await mkdir(directory, { recursive: true, mode: 0o700 });
  if (createdDirectory) await chmod(directory, 0o700);
  const target = join(directory, FILE);
  const temporary = `${target}.${randomUUID()}.tmp`;
  const handle = await open(temporary, "wx", 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(state)}\n`, "utf8");
    await handle.sync();
  } finally {
    await handle.close();
  }
  await rename(temporary, target);
  await chmod(target, 0o600);
  return state;
}

export function publicCloudStatus(state) {
  const connection = state.connection;
  return connection ? {
    paired: true,
    cloudUrl: connection.cloudUrl,
    hostId: connection.hostId,
    workspaceId: connection.workspaceId,
    lastAttemptAt: connection.lastAttemptAt,
    lastSuccessAt: connection.lastSuccessAt,
    lastErrorCode: connection.lastErrorCode,
  } : { paired: false };
}
