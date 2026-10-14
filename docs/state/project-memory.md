# Project memory feature state

Status: Slice A complete; decision/index/agent/extraction slices remain planned

Last updated: 2026-10-14

## Owner selected outcome

Multiple agents or teams share project-scoped local memory. Patchfleet project
messages, tasks, and owner answers are the default sources. External agent
conversations are optional imports. Automatic extraction creates ADR drafts;
only explicit owner acceptance makes a decision authoritative.

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

## Verified evidence

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

## Next

Verify decision proposal/acceptance, embedded
database packaging and accepted-decision retrieval before
adding real agent tools and automatic extraction.

## Current limits

No memory database, embedding model, accepted-decision lifecycle, MCP memory tools
or automatic ADR extractor is enabled. External conversations and historical
unbound work are not imported. Cloud-created teams do not get implicit bindings;
owner replies on an already locally bound team retain its historical scope and
are labeled `owner` rather than claiming a local author. Deletion/retention controls
are deferred; unlinking is not forgetting. No provider settings, transcript import,
live Cloud account or installed owner data was modified by this slice.
