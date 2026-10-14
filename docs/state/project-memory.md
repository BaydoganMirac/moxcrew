# Project memory feature state

Status: Planned

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
  accepted Rust/Tauri host. Its migration is implemented under Task 0023.

## Next

Prepare Slice A in the Rust host: stable Project identity, explicit workspace
linkage, durable Patchfleet source capture, compatibility and isolation checks.
Then verify embedded database packaging and accepted-decision retrieval before
adding real agent tools and automatic extraction.

## Current limits

No Project domain, memory database, embedding model, MCP memory tools or
automatic ADR extractor has shipped. The Rust host is implemented, but this
memory plan remains a separate feature phase. Owner-selected sources and
explicit ADR acceptance remain binding product rules. No provider settings,
transcript import or live Cloud account was modified by the migration.
