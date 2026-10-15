# Project memory feature state

Status: Local ADR lifecycle and explicit paid Cloud publication independently accepted

Last updated: 2026-10-15

## Owner selected outcome

Multiple agents or teams share project-scoped local memory. Patchfleet project
messages, tasks, and owner answers are the default sources. External agent
conversations are optional imports. Automatic extraction creates ADR drafts;
only explicit owner acceptance makes a decision authoritative.

[ADR 0029](../decisions/0029-free-local-and-subscription-cloud.md) adds the
owner-selected commercial boundary: Local and its planned memory capabilities
remain free; optional hosted/shared memory belongs to subscription Cloud.
Explicit opt-in publication now has a separate Memory protocol1 and private
PostgreSQL implementation, with the single-publisher rule from ADR 0030/0032.
Pgvector, pricing, commercial allowances and embedding choice remain later work.
Only the new memory service requires verified paid entitlement; existing Cloud
Free/Solo behavior has not undergone the broader commercial transition. No
default upload boundary changed, and no deployed database was migrated.

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

## Explicit subscription Cloud publication — Task 0031

[Task 0031](../plans/0031-cloud-memory-publication.md) implements approved
[ADR 0032](../decisions/0032-opt-in-shared-memory-transport.md). V5 canonical
facts preserve owner queue consent, the frozen outbound DTO, delivery attempts
and server receipts. The mapping binds URL/workspace/host; a connection change
cannot silently reuse prior upload consent. Queued is distinct from published.
An uncertain attempt retries the same operation, including after restart or a
later Local change, to recover its original acknowledgement. An unattempted
publication is canceled if its accepted Local revision is no longer current.

The Cloud view explicitly creates a publisher project or links a reader in the
same personal workspace. The Decisions view previews accepted ADR fields and
requires a fresh upload checkbox. Source bodies/origin events remain Local;
only opaque source ID/revision references accompany the selected accepted text.
Pairing, payment, acceptance and source editing never publish automatically.
Downloaded Cloud records are read-only copies, separate from Local acceptance.
The UI identifies current/historical, stale/offline, last verification, pending
publication, subscription/access errors and changed pairing in Turkish/English.

Downloads validate complete consistent manifests and immutable exact records
before replacing a project view. The protected derived cache is capped at
16 MiB, 64 current heads and 128 retained versions per project. Limits are
technical safeguards, not commercial allowances. Delivery uses existing Rust
HTTP dependencies, at most four requests and three seconds per tick, with close
cancellation. No database daemon, vector engine or new production dependency
was added to the desktop. Replay still scales with canonical history.

The real cross-repository fixture runs two Rust Hosts through HTTP to a fresh
native PostgreSQL16 cluster. It proves the identical accepted ID/revision/body,
lost acknowledgement and restart recovery, atomic published replacement,
sharing-off denial with stale offline copies retained, inactive-subscription
denial and continued free Local commands. A database-wide source canary proves
raw evidence was not uploaded. The existing normalized PostgreSQL test passes
after all four migrations run twice on a fresh legacy seed; no owner database
or live billing/provider system is involved. See native state for final UI and
development package evidence.

Final verification passes 158 Rust tests (104 core and 54 desktop), 149 legacy
tests and 38 private Cloud tests. Two ordinary-suite PostgreSQL cases are
intentionally skipped there and both pass through separate fresh-database
runners; the actual two-Rust-Host HTTP/PostgreSQL proof also passes against final
code. One pre-existing manual large-log probe remains ignored. Workspace
formatting and strict all-target/all-feature Clippy pass; all 246 original V1
prefix hashes are preserved. Core delivery is `cab9bc8`, native UI `1d44b55`,
queue-cap repair `bea89cd`, terminal-rejection recovery `46e149d`, and coordinator
integration `896c3bd`.

Independent review closed three concrete issues: expired reader-link retries now
recover their original receipt without granting access; remapping a completed
old connection cannot exceed 32 pending operations; valid capacity/immutable/
overflow rejections produce terminal receipts instead of trapping the queue.
Lost rejection acknowledgements survive restart and permit a new replacement or
sharing change after recovery. Unknown bare HTTP409 responses remain uncertain;
no client guesses a terminal result. Semantic rejection feedback remains visible
without marking a fresh verified cache stale. Review accepted the final core/UI
and reduced transport contract with no remaining concrete code blocker.

## Next

The staged [Task 0029](../plans/0029-decision-memory-and-cloud-publication.md)
turns the free Local/subscription Cloud decision into bounded implementation
slices. Following explicit Cloud publication, Cloud vectors, embedded Local
retrieval evaluation, scoped agent tools
and automatic extraction follow working exact decision retrieval.

## Current limits

[Task0032](project-memory-search.md) now implements optional Cloud vector retrieval and free Local ranked keyword search. Task0034 now enables scoped read-only Local MCP tools; an automatic ADR extractor is not enabled. Cloud publication code requires compatible service deployment,
explicit consent and verified subscription. Owned Codex work and team roles now receive current accepted Local ADR context through bounded tools. See [agent state](agent-memory.md). External conversations and historical
unbound work are not imported. Cloud-created teams do not get implicit bindings;
owner replies on an already locally bound team retain its historical scope and
are labeled `owner` rather than claiming a local author. Deletion/retention controls
are deferred; unlinking is not forgetting. No provider settings, transcript import,
live Cloud account or installed owner data was modified by this slice.
