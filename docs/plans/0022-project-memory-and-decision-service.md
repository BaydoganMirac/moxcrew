# Task 0022 Project memory and decision service

Status: Slice A complete; remaining slices ready for separate implementation cards

Owner direction: 2026-10-14

Product contract: [ADR 0021](../decisions/0021-project-scoped-local-decision-memory.md)

Runtime contract: [ADR 0023](../decisions/0023-rust-tauri-desktop-runtime.md)

This plan defines the requested outcome and separately verifiable slices.
Runtime implementation and package selection are not recorded as completed.

## User outcome

The owner opens a project, starts individual agents or teams, and gives project
guidance once. Patchfleet captures that guidance locally, drafts supported
architecture decisions, and makes explicitly accepted decisions available to
every authorized agent working on that project.

## Example journey

1. The owner registers a project and associates two worktrees with it.
2. The owner writes that invoice calculation must use integer minor units.
3. Extraction proposes a decision with its source message and explanation.
   A vague question about rounding is retained as context, not an accepted ADR.
4. The owner accepts the proposed decision in Patchfleet.
5. A backend agent and a QA team in different worktrees retrieve that same
   decision, including its identity, status, revision, and source references.
6. A later contradictory suggestion is proposed as a conflict or replacement;
   it cannot silently supersede the accepted rule.

## Product objects

| Object | Responsibility |
|---|---|
| Project | Stable local identity, owner, registered workspaces and memory revision |
| Workspace | Existing execution location, linked to a project |
| ProjectSource | Owner input, source kind, identity, revision, actor and local text |
| MemoryEntry | Typed requirement, preference, question or decision context with provenance |
| Decision | Proposed or accepted ADR with context, decision, rationale and consequences |
| DecisionRelation | Conflict, duplicate candidate or accepted supersession relation |
| MemoryIndex | Local searchable text, metadata and model-versioned vectors |
| AgentMemoryBinding | Project, run, capabilities, expiry and revocation state |

Existing team and work schemas require versioned linkage. Do not silently add
fields to strict V1 records or leak project text into existing Cloud projections.
Removing a workspace must not delete the project's accepted decisions.

## Source and extraction contract

Default sources are project messages entered in Patchfleet, work instructions,
team goals, owner answers, and owner decision notes. Preserve source identity
and revision so a retried ingestion does not create duplicate records. Provider
observation remains metadata-only. External conversations require an explicit
project import; never scrape provider homes or transcripts by default.

The extractor receives a bounded source batch and relevant existing decisions.
It returns validated proposals with supporting excerpts; it may return no ADR.
An ordinary question, an agent opinion, or an ambiguous statement is not owner
acceptance. Supported source IDs must resolve within the same project. A source
edit invalidates outdated extraction jobs before they publish their proposals.

Use a durable job identity derived from project, source ID/revision, and
extractor version. Provider unavailability leaves a visible queued or failed
extraction job; it does not lose the owner's source input. The initial provider
integration can reuse a bounded supported Codex run, but must prove structured
output and no workspace writes before automatic extraction is enabled.

## Runtime foundation for Slice A

The owner selected the Rust core in Task 0023 and a native Rust/egui interface in Task 0024. The
local host migration is implemented, with macOS development packaging verified
and native platform release checks tracked separately. New memory slices belong
to that Rust host and its single canonical writer; no Node memory subsystem or
second database authority is introduced.

Slice A is implemented under Task 0025: stable Project identity, explicit workspace
linkage, durable owner sources and a native memory screen. Turkish/English display
support is also verified. The remaining decision/index, MCP and automatic extraction
slices are separate work and are not implied by the migration or source foundation.

## Local storage choice

Evaluate embedded LanceDB in the selected host. Its native Rust crate runs in
process with a local directory; its Node SDK offers the equivalent embedded
shape for the existing product. Neither requires a separate Python server.
Confirm filtering, upsert, index rebuild, crash recovery, native dependencies,
and macOS/Windows packaging before selecting a release version. A Node path
also requires clean tarball loading and the supported Node 22/24 matrix.

SQLite plus sqlite-vec is an alternative if its extension loading and package
matrix are simpler in a measured spike. Do not assume every supported Node 22
version has the same built-in SQLite extension support. A database requiring a
second always-running server is outside the first slice.

Keep canonical events and derived database writes ordered by project memory
revision. Store full searchable ADR text and metadata alongside the vectors.
An index failure must not lose an acknowledged source or acceptance. Rebuild
from canonical facts, with a visible indexing cursor and idempotent record keys.
Vectors generated by different models or dimensions cannot share an index.

Choose a local multilingual embedding model using Turkish/English decision
fixtures. Initial model download is explicit; subsequent search uses the local
model. Index/query prefixes and normalization must follow its documented
contract. Keyword fallback is reported as keyword search, not semantic search.

## Agent access contract

Proposed initial MCP tools:

- `project_context`: current project facts, active decision revision, and a
  manifest of mandatory decisions with pagination when needed.
- `memory_search`: bounded hybrid retrieval with source, status, revision,
  match mode and any index-lag indicator.
- `decision_read`: exact record and provenance by opaque ID.
- `decision_propose`: optional capability to submit a source-backed draft.

All tools use the project's authenticated run binding. No agent-controlled
project path, SQL predicate, arbitrary filesystem read, or acceptance tool is
exposed. Reject cross-project IDs even when their vectors are similar. Binding
expiry and run completion revoke access; credentials never enter prompts,
receipts, exports, or Cloud payloads.

Use a thin local stdio MCP bridge to the owner host when supported, so provider
clients do not need a public network endpoint. Keep database ownership in the
host. The bridge and owner UI call the same service authorization boundary;
MCP transport itself is not authorization. Verify each provider's integration
surface before claiming access is automatic.

Retrieval returns evidence, not new system instructions. Drafts and imported
text are labeled data and cannot grant permissions. Binding decisions use
explicit status and scope rather than vector score. Before implementation or
review, agents must load the required decision manifest; a truncated context
cannot silently omit a mandatory rule. Record its memory revision in the task.
Changed decisions mark downstream context stale for refresh or owner review.

## Implementation order

| Slice | Observable outcome | Acceptance boundary |
|---|---|---|
| A Project and sources | One project links multiple workspaces and captures owner inputs durably | Restart, source revision, duplicate ingestion, project isolation and old workspace compatibility |
| B Decisions and local index | Owner proposes, accepts and supersedes a decision; local search returns the correct active record | Exact read, conflict preservation, index rebuild, model mismatch, Turkish/English retrieval and Cloud canaries |
| C Agent access | An agent and a team in separate worktrees read the same accepted decision through scoped tools | Real provider MCP handshake, expired/revoked binding, cross-project denial and bounded responses |
| D Automatic drafts | A Patchfleet message produces a sourced draft without changing active decisions | Structured output, source validation, deduplication, source edit race, no-ADR case and extraction failure recovery |

With the Rust foundation selected, Slice A was completed in
[Task 0025](0025-project-source-memory-foundation.md). Its scope is stable Project
identity, workspace linkage, source ingestion, replay and a native UI; it required
no embedding dependency or model API key. Decision/index work needs its own
approved card and exact writer/index/UI/behavior-test ownership before starting.

Each slice produces a feature-state update and a reviewable local diff. Only
broaden checks after a new failure or dependency warrants it. Do not build all
four slices as an unreviewable change.

## Completion criteria

- Two projects cannot access each other's records, including through forged
  IDs, semantic matches, cached context, exports, or raw MCP arguments.
- Two worktrees and two teams in one project share the same accepted decision
  revision without sharing their writable checkouts.
- Extraction always leaves drafts; only owner commands activate decisions.
- Duplicate extraction and command retries preserve the original outcome.
- Conflicts, rejections and superseded records cannot silently become binding.
- Provider outage, embedding outage, index lag and restart have distinct states
  while canonical source and decision records remain recoverable.
- Accepted decisions can be read exactly without semantic retrieval.
- Memory contents, snippets, vectors, queries and credentials never appear in
  default Cloud payloads; MCP does not widen provider permissions.

## Remaining choices

The owner has selected the source boundary and explicit-acceptance policy.
The host runtime is Rust. Database version, embedding model, final source-size
limits, agent credential lifecycle and retention/deletion mechanics remain
implementation decisions.
Deletion must cover canonical data, vector rows, snapshots and exports; removing
only a vector is not a forget operation. No deletion feature is claimed here.

## References

- [LanceDB Rust crate](https://docs.rs/lancedb/latest/lancedb/)
- [LanceDB Node SDK](https://github.com/lancedb/lancedb/blob/main/nodejs/README.md)
- [sqlite-vec Node bindings](https://alexgarcia.xyz/sqlite-vec/js.html)
- [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
