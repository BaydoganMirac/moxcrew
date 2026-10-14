# Task 0025 Project and source memory foundation

Status: Approved — implementing

Owner direction: 2026-10-14, “Onaylı yap”

Implements Slice A of Task 0022 under ADR 0025 and the native runtime in ADR 0024.

## Outcome

The owner creates a stable project, links multiple registered worktrees, writes
project messages, and inspects captured task instructions, team goals, answers
and decision notes in a native Project Memory screen. Restart, retries and changing
workspace links preserve provenance and project isolation.

## Shared contract

New module `project_memory` owns schemaVersion 3 payload validation, deterministic
replay, pure command planning, scoped reads and compact snapshot metadata.
Projection key `projectMemory`, cache file `project-memory.json`:

- `{schemaVersion:3, revision, projects:[], sources:[], bindings:[]}`.
- Project: `{projectId, name, createdAt, revision, workspaceIds:[], sourceCount}`.
- Source: `{sourceId, projectId, kind, title, text, revision, actorId, createdAt,
  updatedAt, origin:{type,id,eventId}}`.
- Binding: `{projectId, subjectKind:"work"|"team", subjectId}`.
- Project/source IDs use `project:<uuid>` / `source:<uuid>`.
- Kinds: `message`, `work_instruction`, `team_goal`, `owner_answer`, `approval_note`.
- Project/source titles 160 UTF-16 units, project names 80; source text 50,000.

Owner actions (normal commandId/createdAt/expiresAt envelope):

- `project_create {name}`.
- `project_link_workspace {projectId, workspaceId, expectedProjectRevision}`.
- `project_unlink_workspace {projectId, workspaceId, expectedProjectRevision}`.
- `project_message {projectId, title, text, expectedProjectRevision}`.
- `project_source_edit {projectId, sourceId, title, text, expectedSourceRevision}`
  (message sources only).
- `project_memory_read {projectId, offset, limit, query}` returns
  `{reasonCode:"PROJECT_MEMORY_READ", projectId, revision, offset, total, sources:[]}`.
  Offset defaults 0, limit 10 and max20, query defaults empty (max160).

Module API:

- `validate_event(&Value) -> Result<()>` (V3 payload plus envelope rules).
- `project(events: &[Value]) -> Result<Value>` (also consumes legacy facts).
- `apply(events: &[Value], action: &str, payload: &Value) -> Result<(Vec<Value>, Value)>`
  (atomic request/fact/receipt, duplicate same receipt, expiry and stale rejection).
- `recover(events: &[Value]) -> Result<Vec<Value>>` (finish partial pure commands).
- `bind(projectId: &str, subjectKind: &str, subjectId: &str) -> Value`.
- `metadata(projection: &Value) -> Value` (projects/revision only).
- `read(projection: &Value, payload: &Value) -> Result<Value>`.

Events: `project.command.requested`, `project.command.receipted`, `project.created`,
`project.workspace.linked`, `project.workspace.unlinked`, `project.source.created`,
`project.source.edited`, `project.subject.bound`. Fact payloads are defined by the
builder, strictly validated and linked to their originating request where relevant.
Read requests are ephemeral and do not log query text. Automatic sources derive
only from explicitly bound work/team facts. Binding survives unlink/removal; manual
work retry inherits the original binding, including remaining unbound if original
work had no project. New local teams are bound; Cloud-created teams remain unbound.
Captured source text is never mutated by an edit command.

Host snapshots use a Store accessor excluding full `projectMemory` then inject
compact metadata. Core scheduling also uses the compact accessor; full source
state is obtained only through the scoped Store memory read. Desktop bridge caches
the selected bounded page in `snapshot.memoryPage`, invalidates stale project
revisions, and surfaces read outcomes without persisting read queries.

## Ownership

- Coordinator: this card, ADR/state/docs; `host.rs`, `work.rs`, `teams.rs`,
  desktop `bridge.rs`/`main.rs`/`command.rs`, integration tests, version/build/package verification.
- Runtime builder: new `project_memory.rs`, `domain.rs`, `store.rs`, `lib.rs`,
  new `tests/project_memory.rs`, narrowly affected projection/parity tests.
- Console builder: desktop `ui.rs` (native projects/linking/memory, UI tests).
- Reviewer: stable implementation commit, read-only durability/isolation/Cloud audit.

Workers read AGENTS and applicable runtime/team skills, preserve V1/V2 contracts,
coordinate conflicting shared shapes, stage only owned files, and report checks and
limits. State/handoff documentation is coordinator-owned for this bounded split.
No provider adapters, private Cloud repository, installed app/data, transcripts,
credentials, pushing or publishing are in scope.

## Acceptance

- One project links two workspaces; one workspace cannot link to two projects.
- Existing unlinked workspaces and all legacy V1 projections retain behavior.
- Message/edit/restart/replay preserves identity, text and revision history.
- Duplicate commands preserve their original receipt and never duplicate sources.
- Stale project/source revisions, forged cross-project source IDs and invalid
  schema/text/limits fail before mutation.
- Task retries and team answers keep historical scope after relink/removal;
  generated team instructions are not captured as owner input.
- Crash-prefix recovery gives each accepted command a terminal outcome.
- Memory contents/IDs/query canaries never enter V1/V2 Cloud projections.
- Native project/message/edit/link/paging actions have bounded, usable feedback.
- Rust tests/fmt/clippy and legacy regression checks pass; actual macOS native
  rendering and a fresh development package are verified. Other platforms remain
  native CI validation requirements.

## Deferred

Vectors, accepted ADRs, automatic extraction, provider MCP access, imports,
deletion/retention controls and real model calls remain later Task 0022 slices.
