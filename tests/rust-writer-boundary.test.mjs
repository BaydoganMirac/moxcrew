import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { appendEvents, replayEvents, rebuildProjection } from "../lib/runtime/observation-store.mjs";

test("legacy writer refuses a Rust-owned data directory", async () => {
  const dataDir = await mkdtemp(join(tmpdir(), "patchfleet-writer-boundary-"));
  try {
    await writeFile(join(dataDir, "writer.lock"), "");
    await writeFile(join(dataDir, "events.jsonl"), "");
    await assert.rejects(appendEvents([], { dataDir }), { code: "RUST_HOST_OWNS_DIRECTORY" });
    await assert.rejects(replayEvents({ dataDir }), { code: "RUST_HOST_OWNS_DIRECTORY" });
    await assert.rejects(rebuildProjection({ dataDir }), { code: "RUST_HOST_OWNS_DIRECTORY" });
    assert.equal(await readFile(join(dataDir, "events.jsonl"), "utf8"), "");
  } finally { await rm(dataDir, { recursive: true, force: true }); }
});
