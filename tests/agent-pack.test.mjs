import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { BUILT_IN_AGENT_PACKS } from "../lib/agents/built-in-packs.mjs";
import { validateAgentPack } from "../lib/domain/agent-pack.mjs";
import {
  installAgentPack,
  installAgentPackFile,
  listAgentPacks,
  readAgentPackProjection,
  removeAgentPack,
} from "../lib/runtime/agent-pack-registry.mjs";

function custom(version = "1.0.0") {
  return {
    schemaVersion: 1,
    id: "pack:customer-maintainer",
    version,
    name: "Customer Maintainer",
    role: "fullstack",
    description: "Maintains one customer application with bounded changes.",
    providerId: "codex",
    instructions: "Implement the assigned acceptance criteria and return test evidence.",
    requiredCapabilities: ["work.start", "work.cancel"],
    permissions: ["read_workspace", "write_workspace", "run_checks"],
    defaultModel: null,
    limits: { maxAttempts: 2, timeoutMinutes: 45 },
    expectedOutput: "Outcome, changed files, checks, blockers, and handoff.",
    qualityChecks: ["tests", "build"],
    provenance: { kind: "local", source: "customer-maintainer.json" },
  };
}

test("built-in catalog provides validated immutable ready roles", () => {
  assert.equal(BUILT_IN_AGENT_PACKS.length, 12);
  assert.deepEqual(new Set(BUILT_IN_AGENT_PACKS.map((pack) => pack.role)), new Set([
    "orchestrator", "product", "design", "frontend", "backend", "fullstack",
    "qa", "review", "security", "release", "docs", "research",
  ]));
  for (const pack of BUILT_IN_AGENT_PACKS) {
    assert.deepEqual(validateAgentPack(pack), pack);
    assert.equal(pack.provenance.kind, "built-in");
    assert.equal(pack.requiredCapabilities.includes("work.start"), true);
  }
  assert.throws(() => {
    BUILT_IN_AGENT_PACKS.push(custom());
  }, TypeError);
});

test("custom packs install, upgrade, rebuild, list, and remove through the durable log", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-agent-packs-"));
  const first = await installAgentPack(custom(), { dataDir, now: () => "2026-07-20T10:00:00.000Z" });
  assert.equal(first.revision, 1);
  await assert.rejects(() => installAgentPack(custom(), { dataDir }), /version must increase/);
  await installAgentPack(custom("1.1.0"), { dataDir, now: () => "2026-07-20T10:01:00.000Z" });

  let projection = await readAgentPackProjection({ dataDir });
  assert.equal(projection.revision, 2);
  assert.equal(projection.items[0].revision, 2);
  assert.equal(projection.items[0].pack.version, "1.1.0");
  await writeFile(join(dataDir, "agent-packs.json"), "broken", "utf8");
  projection = await readAgentPackProjection({ dataDir });
  assert.equal(projection.items[0].pack.version, "1.1.0");
  assert.equal((await listAgentPacks({ dataDir })).length, 13);

  await removeAgentPack(custom().id, { dataDir, now: () => "2026-07-20T10:02:00.000Z" });
  assert.equal((await listAgentPacks({ dataDir })).length, 12);
  const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
  assert.equal(events.filter((event) => event.type === "agent_pack.installed").length, 2);
  assert.equal(events.filter((event) => event.type === "agent_pack.removed").length, 1);
});

test("file installation stores no absolute source path and rejects executable or unsafe manifests", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-agent-pack-file-data-"));
  const files = await mkdtemp(join(tmpdir(), "patchfleet-agent-pack-file-"));
  const filename = join(files, "safe-pack.json");
  const manifest = custom();
  delete manifest.provenance;
  await writeFile(filename, JSON.stringify(manifest), "utf8");
  const installed = await installAgentPackFile(filename, { dataDir });
  assert.deepEqual(installed.pack.provenance, { kind: "local", source: "safe-pack.json" });
  assert.equal((await readFile(join(dataDir, "events.jsonl"), "utf8")).includes(files), false);

  assert.throws(() => validateAgentPack({ ...custom(), executable: "node plugin.js" }), /invalid agent pack/);
  assert.throws(() => validateAgentPack({ ...custom(), id: "pack:../../escape" }), /invalid agent pack/);
  assert.throws(() => validateAgentPack({ ...custom(), instructions: "ok\0bad" }), /invalid agent pack instructions/);
  assert.throws(() => validateAgentPack({ ...custom(), permissions: ["shell"] }), /invalid agent pack permissions/);
  assert.throws(() => validateAgentPack({ ...custom(), limits: { maxAttempts: 999, timeoutMinutes: 1 } }), /invalid agent pack limits/);
  await assert.rejects(() => installAgentPack({ ...custom("2.0.0"), id: "pack:orchestrator" }, { dataDir }), /built-in/);
  await assert.rejects(() => removeAgentPack("pack:orchestrator", { dataDir }), /built-in/);
});
