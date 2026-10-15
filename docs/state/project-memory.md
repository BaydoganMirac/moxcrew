# Project memory feature state

Status: Source foundation and local decision lifecycle implemented and independently accepted

Last updated: 2026-10-15

## Owner selected outcome

Multiple agents or teams share project-scoped local memory. Patchfleet project
messages, tasks, and owner answers are the default sources. External agent
conversations are optional imports. Automatic extraction creates ADR drafts;
only explicit owner acceptance makes a decision authoritative.

[ADR 0029](../decisions/0029-free-local-and-subscription-cloud.md) adds the
owner-selected commercial boundary: Local and its planned memory capabilities
remain free; optional hosted/shared memory belongs to subscription Cloud.
Cloud PostgreSQL/pgvector and project opt-in publication are proposed next work,
not existing memory storage. Pricing, quotas, embedding choice and the new memory
protocol remain unresolved; ADR 0030 proposes the initial single-publisher rule.
No default upload boundary or
runtime entitlement changed.

## Recorded

- [ADR 0021](../decisions/0021-project-scoped-local-decision-memory.md) records
  the product and trust contract.
- [Task 0022](../plans/0022-project-memory-and-decision-service.md) specifies
  Project identity, source provenance, decision lifecycle, embedded vector
  storage evaluation, scoped agent tools, and four bounded slices.
- The current workspace represents one worktree, so shared project identity
  must precede per-project memory and cross-worktree retrieval.
- [ADR 0023](../decisions/0023-rust-tauri-desktop-runtime.md) records the
  accepted Rust core from Task 0023, with the native Rust/egui interface from Task 0024.
- [ADR 0025](../decisions/0025-project-source-memory-foundation.md) and
  [Task 0025](../plans/0025-project-source-memory-foundation.md) define the
  approved project/source implementation slice.

## Slice A implementation

- Stable project IDs explicitly link one or more registered worktrees. A worktree
  has at most one current project. Existing unlinked registrations remain usable.
- Local V3 commands/facts/receipts extend the canonical log without widening V1
  work/team/workspace records or Cloud V1/V2 contracts. Pure project commands
  recover crash prefixes and return their original semantic receipts on retries.
- Project messages retain source identity and append-only revision history.
  Captured work instructions, team goals, answers and approval notes remain
  immutable evidence. Sources do not become accepted architecture decisions.
- Explicit subject bindings freeze the project for locally submitted work/teams.
  Replay derives sources from their V1 facts; generated team prompts stay outside
  owner capture. Task retry inherits its original binding. Relink/removal does not
  move historical sources or erase project history.
- The protected `project-memory.json` projection rebuilds from the log. Operational
  snapshots clone project metadata only. Memory reads are project-specific, max20
  sources per page, with optional keyword filtering; the native UI uses pages of 10.
  Cached pages are invalidated when project revision changes or scope/filter changes.
- Cloud sync uses an event cursor rather than copying source-bearing canonical
  history. Its existing field allowlists omit all memory text, identities and queries.

## Source foundation evidence

- An actual Host integration flow persists all five source kinds, retries owner
  commands without duplicates, relinks a worktree to a second project, verifies
  answers/notes remain in the first, and preserves sources after restart.
- Generated team work is present in the work queue without being captured as a
  separate owner source. Cross-project message edits fail.
- V1/V2 Cloud canaries exclude source text and project identities.
- All 81 Rust tests and 149 legacy tests pass; all 246 original V1 prefix hashes
  remain intact. Rustfmt and strict all-target/all-feature Clippy pass.
- Twelve memory checks cover all five pure command crash prefixes, source edit
  history, same-project access, duplicate commands and recovery across clock
  corrections. Only fresh commands are subject to future-time admission checks.
- Source message/edit intake shares a bounded 192 KiB Host/CLI envelope. Actual
  50,000-unit CJK creation and editing round-trip without truncation.
- Twenty-four native console/bridge/localization checks cover owner clicks,
  stale scope/revision pages, atomic outcome/snapshot delivery and exact retries.
- Turkish/English controls switch through the owner Host and persist in protected,
  bounded `desktop-preferences.json`. Owner/custom text remains original even when
  it matches a translation key. Invalid/damaged preferences preserve canonical data.
- Independent review confirmed the clock/intake fixes, historical project scope,
  source immutability, Cloud allowlists and translation boundaries.
- Actual native macOS ARM64 [Turkish](../screenshots/native-memory-0.5-tr.png) and
  [English](../screenshots/native-memory-0.5-en.png) frames render real disposable
  Host source records. Version 0.5 development App/DMG, signature, identifier and
  image integrity are verified; no installed app or owner data was modified.

## Local decision lifecycle — Task 0030

[Task 0030](../plans/0030-local-decision-lifecycle.md) was approved by the owner
after Task 0029's implementation sequence. [ADR 0031](../decisions/0031-versioned-local-decision-events.md)
implements V4 owner commands/facts/receipts with immutable proposed ADR bodies,
explicit acceptance/rejection and atomic supersession. A new proposed replacement
becomes accepted in the same fact that supersedes its accepted predecessor.
Decision manifests have independent per-project revisions; V3 source revisions
and all original V1/V2 meanings remain intact.

Proposals cite one to eight current source ID/revisions and frozen origin.
Acceptance rechecks freshness. A later source edit does not change accepted
text or original evidence. The protected `project-decisions.json` projection
stores each cited source version once and rebuilds from canonical history.
List reads return bounded summaries; exact reads return immutable content,
original sources and lifecycle history. Queries/reads create no events.
Ordinary Host snapshots clone counts/revisions only, and Cloud V1/V2 allowlists
exclude all decision content and identities. Replay and protected caches still
scale with retained history; no index, model or background database was added.

Core commit `9ab17bb` adds 14 focused decision checks: all four mutation crash
prefixes, durable exact retries/expiry/clock changes, stale source rejection,
historical evidence, interleaved receipt recovery, project isolation and forged
facts/atomic replacement denial. Two real Host checks additionally prove
restart, unchanged Cloud canaries and maximum multilingual ADR intake. All 81
core tests pass (one existing manual large-log probe ignored); all 246 original
V1 prefix hashes and 149 legacy tests pass. Native commit `6aa36d7` adds a
Turkish/English Decisions tab with project/status/keyword filters, source
selection, immutable detail/history and explicit lifecycle actions. All 42
desktop tests pass, including stale/busy/foreign-page denial, exact read retries,
applied-only draft reset, untranslated evidence and minimum-window navigation.
123 Rust tests pass overall, with strict workspace Clippy and formatting.

Independent review found two recovery UI hazards. `f4bc387` keeps newly edited
draft fields/citations when an old exact proposal retry succeeds, and holds
read scheduling/controls without consuming request keys while a mutation error
needs owner recovery. `5b88067`/`d652bf9` prioritize the recovery action and
protect the Bridge's original pending identity/feedback from reads. A real
derived-cache write failure test proves the durable proposal can be retried
without any new canonical bytes; editor/epoch tests preserve later owner edits.
Independent review accepted both fixes through `d652bf9`; no concrete code
blocker remains. The development package was rebuilt and reverified afterward.

Real native TR/EN/minimum-window captures use disposable Host ADRs in all four
states. The feature-off 0.6.0 App/DMG has verified bundle version/identifier,
ad hoc signature, purpose keys, no smoke controls and valid image integrity.
Production NSWorkspace launch, its own window/PID, normal close and writer
release pass; final OS pixel capture was unavailable. See
[native state](native-rust-desktop.md) for checksums, captures and release limits.

## Next

The staged [Task 0029](../plans/0029-decision-memory-and-cloud-publication.md)
turns the free Local/subscription Cloud decision into bounded implementation
slices. Following local accepted decisions, the next concrete card covers
explicit Cloud publication and the same exact revision read on a second installation.
[ADR 0030](../decisions/0030-shared-decision-publication.md) proposes one paired
publishing host per shared project. These technical choices are proposed, not
enabled. Cloud vectors, embedded Local retrieval evaluation, scoped agent tools
and automatic extraction follow working exact decision retrieval.

## Current limits

No memory database, embedding model, MCP memory tools, automatic ADR extractor
or Cloud publication is enabled. Agent context is not automatically populated
from accepted ADRs yet. External conversations and historical
unbound work are not imported. Cloud-created teams do not get implicit bindings;
owner replies on an already locally bound team retain its historical scope and
are labeled `owner` rather than claiming a local author. Deletion/retention controls
are deferred; unlinking is not forgetting. No provider settings, transcript import,
live Cloud account or installed owner data was modified by this slice.
