import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, realpath, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import test from "node:test";

const exec = promisify(execFile);
const cli = join(process.cwd(), "bin", "patchfleet.mjs");

test("package metadata exposes the supported Node CLI release surface", async () => {
  const packageJson = JSON.parse(await readFile("package.json", "utf8"));
  assert.equal(packageJson.private, undefined);
  assert.equal(packageJson.bin.patchfleet, "bin/patchfleet.mjs");
  assert.equal(packageJson.engines.node, ">=22.0.0");
  assert.equal(packageJson.repository.url, "git+https://github.com/BaydoganMirac/patchfleet.git");
  assert.equal(packageJson.publishConfig.access, "public");
  assert.equal(packageJson.homepage, "https://github.com/BaydoganMirac/patchfleet#readme");
  assert.equal(packageJson.bugs, "https://github.com/BaydoganMirac/patchfleet/issues");
  assert.equal(packageJson.dependencies.next, "15.5.19");
  assert.equal(packageJson.scripts.prepack, "npm run build");
  assert.match(packageJson.scripts.build, /sanitize-package/);
  assert.doesNotMatch(packageJson.scripts.build, /--turbopack/, "production start needs the complete Next routes manifest");
  assert(packageJson.files.includes(".next"));
  assert.equal(packageJson.files.includes("tests"), false);
  const publishWorkflow = await readFile(".github/workflows/publish.yml", "utf8");
  assert.match(publishWorkflow, /id-token: write/);
  assert.match(publishWorkflow, /npm@11\.5\.1/);
  assert.match(publishWorkflow, /npm publish --access public --tag beta/);
});

test("CLI help, diagnostics, stopped status, stale metadata cleanup, and empty recovery are safe", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-cli-"));
  const env = { ...process.env, PATCHFLEET_DATA_DIR: dataDir, PATCHFLEET_PORT: "39123" };
  assert.match((await exec(process.execPath, [cli, "--help"], { env })).stdout, /start\|stop\|status\|doctor\|recover\|workspace\|agent-pack/);
  const diagnosed = await exec(process.execPath, [cli, "doctor"], { env });
  assert.match(diagnosed.stdout, /Doctor found no blocking problems/);
  assert.equal(diagnosed.stdout.includes(dataDir), false);
  assert.equal(diagnosed.stdout.includes("credential"), false);
  assert.match((await exec(process.execPath, [cli, "status"], { env })).stdout, /stopped/);

  await writeFile(join(dataDir, "runtime.json"), `${JSON.stringify({
    schemaVersion: 1,
    runtimeId: "runtime:00000000-0000-4000-8000-000000000000",
    pid: process.pid,
    port: 39123,
    startedAt: "2026-07-17T00:00:00.000Z",
    version: "0.1.0",
  })}\n`, { mode: 0o600 });
  assert.match((await exec(process.execPath, [cli, "status"], { env })).stdout, /stopped/);
  await assert.rejects(() => readFile(join(dataDir, "runtime.json")), { code: "ENOENT" });
  assert.match((await exec(process.execPath, [cli, "recover"], { env })).stdout, /recovered/);
});

test("doctor fails closed on corrupt durable state without disclosing it", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-cli-doctor-"));
  const env = { ...process.env, PATCHFLEET_DATA_DIR: dataDir };
  await writeFile(join(dataDir, "events.jsonl"), "FORBIDDEN_PROMPT_CANARY\n", { mode: 0o600 });
  await assert.rejects(
    () => exec(process.execPath, [cli, "doctor"], { env }),
    (error) => error.code === 1 && /Durable state: event log or a derived projection is corrupt/.test(error.stdout) &&
      !error.stdout.includes("FORBIDDEN_PROMPT_CANARY"),
  );
});

test("CLI adds, lists, and removes a local workspace", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-cli-workspaces-"));
  const workspace = await mkdtemp(join(tmpdir(), "patchfleet-cli-repo-"));
  await mkdir(join(workspace, ".git"));
  const env = { ...process.env, PATCHFLEET_DATA_DIR: dataDir };

  const added = (await exec(process.execPath, [cli, "workspace", "add", "."], { env, cwd: workspace })).stdout;
  assert.match(added, /Registered patchfleet-cli-repo-/);
  const workspaceId = added.match(/workspace:[0-9a-f-]{36}/i)?.[0];
  assert.ok(workspaceId);

  const duplicate = (await exec(process.execPath, [cli, "workspace", "add", workspace], { env })).stdout;
  assert.match(duplicate, /already registered/);
  const listed = (await exec(process.execPath, [cli, "workspace", "list"], { env })).stdout;
  assert.match(listed, new RegExp(workspaceId));
  const canonicalWorkspace = await realpath(workspace);
  assert.match(listed, new RegExp(canonicalWorkspace.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));

  assert.match((await exec(process.execPath, [cli, "workspace", "remove", workspaceId], { env })).stdout, /Removed/);
  assert.match((await exec(process.execPath, [cli, "workspace", "list"], { env })).stdout, /No registered workspaces/);
});

test("CLI lists built-in packs and installs, shows, and removes a custom pack", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-cli-packs-"));
  const env = { ...process.env, PATCHFLEET_DATA_DIR: dataDir };
  const listed = (await exec(process.execPath, [cli, "agent-pack", "list"], { env })).stdout;
  assert.match(listed, /Orchestrator\tpack:orchestrator\t1\.0\.0/);
  const filename = join(dataDir, "pack.json");
  await writeFile(filename, JSON.stringify({
    schemaVersion: 1,
    id: "pack:cli-maintainer",
    version: "1.0.0",
    name: "CLI Maintainer",
    role: "fullstack",
    description: "Maintains a project through the Patchfleet command line.",
    providerId: "codex",
    instructions: "Implement assigned criteria and return verification evidence.",
    requiredCapabilities: ["work.start", "work.cancel"],
    permissions: ["read_workspace", "write_workspace", "run_checks"],
    defaultModel: null,
    limits: { maxAttempts: 2, timeoutMinutes: 30 },
    expectedOutput: "Outcome, files, checks, blockers, and handoff.",
    qualityChecks: ["tests"],
  }), "utf8");
  assert.match((await exec(process.execPath, [cli, "agent-pack", "install", filename], { env })).stdout, /Installed CLI Maintainer/);
  assert.match((await exec(process.execPath, [cli, "agent-pack", "show", "pack:cli-maintainer"], { env })).stdout, /"provenance"/);
  assert.match((await exec(process.execPath, [cli, "agent-pack", "remove", "pack:cli-maintainer"], { env })).stdout, /Removed/);
});
