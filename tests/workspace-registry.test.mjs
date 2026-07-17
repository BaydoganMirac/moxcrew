import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, realpath, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import test from "node:test";
import {
  applyWorkspaceCommand,
  readWorkspaceProjection,
  registerWorkspace,
  removeWorkspace,
} from "../lib/runtime/workspace-registry.mjs";

test("workspace registry is canonical, idempotent, rebuildable, and removable", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-workspaces-data-"));
  const workspace = await mkdtemp(join(tmpdir(), "patchfleet-workspace-"));
  await mkdir(join(workspace, ".git"));
  const now = () => "2026-07-17T12:00:00.000Z";
  const commandId = "cmd:11111111-1111-4111-8111-111111111111";
  const workspaceId = "workspace:22222222-2222-4222-8222-222222222222";
  const canonicalWorkspace = await realpath(workspace);

  const registered = await registerWorkspace(join(workspace, "."), {
    dataDir,
    now,
    commandId,
    workspaceId,
  });
  assert.deepEqual(
    [registered.outcome, registered.reasonCode, registered.workspaceId],
    ["applied", "WORKSPACE_REGISTERED", workspaceId],
  );
  const beforeReceipt = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n");
  await writeFile(join(dataDir, "events.jsonl"), `${beforeReceipt.slice(0, -1).join("\n")}\n`, "utf8");
  assert.deepEqual(await registerWorkspace(workspace, {
    dataDir,
    now,
    commandId,
    workspaceId,
  }), registered, "same command repairs a missing receipt without another registration");

  const duplicate = await registerWorkspace(workspace, {
    dataDir,
    now: () => "2026-07-17T12:01:00.000Z",
    commandId: "cmd:33333333-3333-4333-8333-333333333333",
    workspaceId: "workspace:44444444-4444-4444-8444-444444444444",
  });
  assert.deepEqual(
    [duplicate.outcome, duplicate.reasonCode, duplicate.workspaceId],
    ["rejected", "WORKSPACE_ALREADY_REGISTERED", workspaceId],
  );

  let projection = await readWorkspaceProjection({ dataDir });
  assert.equal(projection.revision, 1);
  assert.equal(projection.items.length, 1);
  assert.equal(projection.items[0].displayName, basename(workspace));
  assert.equal(projection.items[0].workingDirectory, canonicalWorkspace);
  assert.equal(projection.receipts.length, 2);

  await writeFile(join(dataDir, "workspaces.json"), "broken", "utf8");
  projection = await readWorkspaceProjection({ dataDir });
  assert.equal(projection.items[0].workspaceId, workspaceId, "stored projection is rebuilt from events");

  const removed = await removeWorkspace(workspaceId, {
    dataDir,
    now: () => "2026-07-17T12:02:00.000Z",
    commandId: "cmd:55555555-5555-4555-8555-555555555555",
  });
  assert.deepEqual([removed.outcome, removed.reasonCode], ["applied", "WORKSPACE_REMOVED"]);
  const missing = await removeWorkspace(workspaceId, {
    dataDir,
    now: () => "2026-07-17T12:03:00.000Z",
    commandId: "cmd:66666666-6666-4666-8666-666666666666",
  });
  assert.deepEqual([missing.outcome, missing.reasonCode], ["rejected", "WORKSPACE_NOT_FOUND"]);

  projection = await readWorkspaceProjection({ dataDir });
  assert.equal(projection.revision, 2);
  assert.deepEqual(projection.items, []);
  assert.equal(projection.receipts.length, 4);
  const events = (await readFile(join(dataDir, "events.jsonl"), "utf8")).trim().split("\n").map(JSON.parse);
  assert.equal(events.filter((item) => item.type === "workspace.registered").length, 1);
  assert.equal(events.filter((item) => item.type === "workspace.removed").length, 1);
  assert.equal(events.filter((item) => item.type === "workspace.command.receipted").length, 4);
});

test("workspace registration rejects unsafe or non-Git directories before persistence", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-workspaces-invalid-data-"));
  const directory = await mkdtemp(join(tmpdir(), "patchfleet-not-git-"));
  await assert.rejects(() => registerWorkspace(directory, { dataDir }), { code: "WORKSPACE_NOT_ALLOWED" });
  await assert.rejects(() => readFile(join(dataDir, "events.jsonl")), { code: "ENOENT" });
});

test("workspace commands reject stale revisions and expire without mutation", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-workspaces-bounds-data-"));
  const workspace = await mkdtemp(join(tmpdir(), "patchfleet-workspace-bounds-"));
  await mkdir(join(workspace, ".git"));
  const canonical = await realpath(workspace);
  const workspaceId = "workspace:77777777-7777-4777-8777-777777777777";
  await registerWorkspace(workspace, {
    dataDir,
    workspaceId,
    commandId: "cmd:77777777-7777-4777-8777-777777777777",
    now: () => "2026-07-17T13:00:00.000Z",
  });

  const stale = await applyWorkspaceCommand({
    schemaVersion: 1,
    intentId: "cmd:88888888-8888-4888-8888-888888888888",
    idempotencyKey: "cmd:88888888-8888-4888-8888-888888888888",
    type: "remove_workspace",
    actorId: "local-owner",
    createdAt: "2026-07-17T13:01:00.000Z",
    expiresAt: "2026-07-17T13:06:00.000Z",
    payload: { workspaceId, expectedWorkspaceRevision: 2 },
  }, { dataDir, now: () => "2026-07-17T13:01:01.000Z" });
  assert.deepEqual([stale.outcome, stale.reasonCode], ["rejected", "STALE_WORKSPACE_REVISION"]);

  const expired = await applyWorkspaceCommand({
    schemaVersion: 1,
    intentId: "cmd:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    idempotencyKey: "cmd:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    type: "register_workspace",
    actorId: "local-owner",
    createdAt: "2026-07-17T13:01:00.000Z",
    expiresAt: "2026-07-17T13:02:00.000Z",
    payload: {
      workspace: {
        schemaVersion: 1,
        workspaceId: "workspace:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
        displayName: "expired",
        workingDirectory: canonical,
        createdAt: "2026-07-17T13:01:00.000Z",
        revision: 1,
      },
    },
  }, { dataDir, now: () => "2026-07-17T13:03:00.000Z" });
  assert.deepEqual([expired.outcome, expired.reasonCode], ["expired", "COMMAND_EXPIRED"]);
  assert.deepEqual((await readWorkspaceProjection({ dataDir })).items.map((item) => item.workspaceId), [workspaceId]);
});
