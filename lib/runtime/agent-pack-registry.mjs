import { randomUUID } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { basename } from "node:path";
import { BUILT_IN_AGENT_PACKS, builtInAgentPack } from "../agents/built-in-packs.mjs";
import { comparePackVersions, projectAgentPackEvents, validateAgentPack } from "../domain/agent-pack.mjs";
import { commitEventTransaction, readAgentPackProjection } from "./observation-store.mjs";

export { readAgentPackProjection };

const MAX_PACK_BYTES = 128 * 1024;

function event(type, recordedAt, payload) {
  return { id: randomUUID(), schemaVersion: 1, type, recordedAt, payload };
}

function nowValue(now) {
  const value = now();
  if (typeof value !== "string" || new Date(value).toISOString() !== value) throw new TypeError("now must return an ISO timestamp");
  return value;
}

export async function installAgentPack(value, { dataDir, now = () => new Date().toISOString() } = {}) {
  const pack = validateAgentPack(value);
  if (pack.provenance.kind !== "local") throw new TypeError("custom pack provenance must be local");
  if (builtInAgentPack(pack.id)) throw new TypeError("built-in pack ids cannot be replaced");
  const recordedAt = nowValue(now);
  return commitEventTransaction((events) => {
    const projection = projectAgentPackEvents(events);
    const current = projection?.items.find((item) => item.pack.id === pack.id);
    if (current && comparePackVersions(pack.version, current.pack.version) <= 0) {
      throw new TypeError("agent pack update version must increase");
    }
    return {
      additions: [event("agent_pack.installed", recordedAt, { pack })],
      result: () => ({ pack, revision: (current?.revision ?? 0) + 1, installedAt: recordedAt }),
    };
  }, { dataDir });
}

export async function installAgentPackFile(filename, options = {}) {
  const info = await stat(filename);
  if (!info.isFile() || info.size < 2 || info.size > MAX_PACK_BYTES) throw new TypeError("agent pack file is invalid or too large");
  let value;
  try {
    value = JSON.parse(await readFile(filename, "utf8"));
  } catch {
    throw new TypeError("agent pack file must contain valid JSON");
  }
  const input = { ...value, provenance: { kind: "local", source: basename(filename) } };
  return installAgentPack(input, options);
}

export async function removeAgentPack(packId, { dataDir, now = () => new Date().toISOString() } = {}) {
  if (builtInAgentPack(packId)) throw new TypeError("built-in packs cannot be removed");
  const projection = await readAgentPackProjection({ dataDir });
  const current = projection?.items.find((item) => item.pack.id === packId);
  if (!current) throw new TypeError("agent pack is not installed");
  const recordedAt = nowValue(now);
  return commitEventTransaction((events) => {
    const latest = projectAgentPackEvents(events)?.items.find((item) => item.pack.id === packId);
    if (!latest || latest.revision !== current.revision) throw new TypeError("agent pack changed; retry removal");
    return {
      additions: [event("agent_pack.removed", recordedAt, { packId, expectedRevision: current.revision })],
      result: () => ({ packId, removedAt: recordedAt }),
    };
  }, { dataDir });
}

export async function listAgentPacks({ dataDir } = {}) {
  const custom = (await readAgentPackProjection({ dataDir }))?.items.map((item) => item.pack) ?? [];
  return [...BUILT_IN_AGENT_PACKS, ...custom].sort((left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id));
}

export async function resolveAgentPack(packId, { dataDir } = {}) {
  return builtInAgentPack(packId) ?? (await readAgentPackProjection({ dataDir }))?.items.find((item) => item.pack.id === packId)?.pack ?? null;
}
