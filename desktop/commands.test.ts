import test from "node:test";
import assert from "node:assert/strict";
import { commandFromForm } from "./commands.ts";

const now = new Date("2026-10-14T10:00:00.000Z");
const id = "00000000-0000-4000-8000-000000000001";

test("work mutations preserve revision and submit-time idempotency envelope", () => {
  assert.deepEqual(commandFromForm("work", {action: "start", workItemId: "work:1", expectedItemRevision: "3"}, now, id), {
    action: "work_start",
    payload: {workItemId: "work:1", expectedItemRevision: 3, commandId: `cmd:${id}`, createdAt: now.toISOString(), expiresAt: "2026-10-14T10:05:00.000Z"},
  });
  assert.throws(() => commandFromForm("work", {action: "cancel", expectedRunRevision: "0"}), /revision/);
  assert.throws(() => commandFromForm("work", {action: "execute_shell"}), /Unsupported/);
  assert.throws(() => commandFromForm("work", {action: "toString"}), /Unsupported/);
  assert.match(String(commandFromForm("observe", {}).payload.commandId), /^cmd:[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
});

test("team builder sends structured worker IDs and bounded numeric settings", () => {
  const command = commandFromForm("team", {action: "create", workerPack1: "pack:qa", workerPack2: "", concurrency: "2", retryLimit: "0", timeoutMinutes: "60", failurePolicy: "stop"}, now, id);
  assert.deepEqual(command.payload.workerPackIds, ["pack:qa"]);
  assert.deepEqual(command.payload.settings, {concurrency: 2, retryLimit: 0, timeoutMinutes: 60, failurePolicy: "stop"});
  assert.equal(command.payload.workerPack1, undefined);
  assert.throws(() => commandFromForm("team", {action: "create", concurrency: "20", retryLimit: "0", timeoutMinutes: "60"}), /team limit/);
  assert.equal(commandFromForm("team", {action: "reject", teamId: "team:1", taskId: "task:1"}).payload.decision, "rejected");
});

test("pack import accepts an explicit JSON object, never a file path", () => {
  assert.deepEqual(commandFromForm("pack", {action: "install", manifest: '{"id":"pack:custom"}'}).payload.manifest, {id: "pack:custom"});
  for (const manifest of ["/tmp/manifest.json", "null", "[]"]) assert.throws(() => commandFromForm("pack", {action: "install", manifest}));
});

test("desktop action routing keeps workspace, work, team and Cloud namespaces separate", () => {
  for (const [namespace, action, expected] of [
    ["observe", "", "observe"], ["workspace", "register", "workspace_register"], ["workspace", "remove", "workspace_remove"],
    ["work", "retry", "work_retry"], ["work", "remove", "work_remove"],
    ["team", "start", "team_start"], ["team", "advance", "team_advance"], ["team", "cancel", "team_cancel"],
    ["team", "cancel_agent", "team_cancel_agent"], ["team", "answer", "team_answer"], ["pack", "remove", "pack_remove"],
    ["cloud", "pair", "cloud_pair"], ["cloud", "disconnect", "cloud_unpair"], ["cloud", "sync", "cloud_sync"],
  ] as const) assert.equal(commandFromForm(namespace, {action}).action, expected);
});
