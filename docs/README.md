# Patchfleet documentation

These documents are written for both humans and coding agents. Product intent
and trust boundaries come before implementation details.

## Read first

1. [Product brief](product.md) — promise, jobs, scope, principles, and success
   criteria.
2. [Architecture](architecture.md) — public/local and private/Cloud
   responsibilities.
3. [Host-to-Cloud protocol](protocol.md) — data policy, pairing, projections,
   command intents, and receipts.
4. [Roadmap](roadmap.md) — public Now / Next / Later technical direction.
5. [Install and operate](install.md) — desktop installation, lifecycle, upgrade,
   recovery, and Gemini extension instructions.
6. [Release](release.md) — native desktop verification, signing, artifacts and rollback.
7. [Agent operating model](agent-operating-model.md) — when parallel agents
   help and how their ownership is bounded.
8. [Project skills](../.agents/skills/) — task-triggered rules for adapters,
   local state, the Cloud boundary, and team delegation.

## Current product focus

[Native Rust interface](plans/0024-native-rust-desktop-ui.md) replaces the Tauri
interface with a lightweight egui desktop. See [native state](state/native-rust-desktop.md)
and [ADR 0024](decisions/0024-native-rust-desktop-ui.md) for current evidence.
The [Rust core migration](plans/0023-rust-desktop-migration.md) remains its foundation.

[Project memory and decision service](plans/0022-project-memory-and-decision-service.md)
defines the planned project-scoped agent/team workflow, automatic sourced ADR
drafts, explicit owner acceptance, and local retrieval tools. See
[ADR 0021](decisions/0021-project-scoped-local-decision-memory.md) for the trust
contract and [feature state](state/project-memory.md) for implementation status.

## Execution records

- [Plans](plans/) describe a bounded implementation slice and its acceptance
  criteria.
- [Decisions](decisions/) explain choices that must survive context loss.
- [State](state/) tells the next agent what is done, active, blocked, and next.

## Public governance

- [Security policy](../SECURITY.md) explains private vulnerability reporting.
- [Apache License 2.0](../LICENSE) defines use and contribution terms.
- [ADR 0007](decisions/0007-public-and-internal-document-boundary.md) defines
  which product information belongs in this public repository.

## Source-of-truth order

When documents disagree, use this order:

1. Accepted ADRs
2. Product and protocol contracts
3. Active implementation plan
4. Feature state
5. Code comments and generated output

Fix the stale document in the same change instead of silently working around a
conflict.
