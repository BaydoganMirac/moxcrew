# Moxcrew documentation

These documents are written for both humans and coding agents. Product intent
and trust boundaries come before implementation details.

Looking for the product overview, screenshots or development preview? Start with
the repository [README](../README.md). This index is the deeper product,
architecture and delivery record.

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
9. [UI/UX reference](../design/moxcrew/UX-RULES.md) — researched standards versus
   product choices, native UI diagnosis, component roles, task flows and
   acceptance rules; read with the [implemented UI kit](../design/moxcrew/UI-KIT.md).

## Current product focus

[Moxcrew 1.0.0](plans/0039-version-one-release.md) is the owner-approved release
target: eight areas, forty abilities and ten composed workflows. The
[release state](state/version-one-release.md) records actual implementation and
remaining evidence; the current application is not yet 1.0.0.

[General-purpose Projects](decisions/0038-general-purpose-projects.md) is the
owner-selected product direction: work beyond software development, with
Project as the primary scope. See [state](state/general-purpose-projects.md)
for the explicit gap between this direction and current Git/Codex execution.

[Moxcrew identity and interface](state/dobbly-brand-interface.md) records the
Mox mascot, system themes, native package and completed local verification.
Technical installation/data identifiers remain compatible with earlier releases.

[Native Rust interface](plans/0024-native-rust-desktop-ui.md) replaces the Tauri
interface with a lightweight egui desktop. See [native state](state/native-rust-desktop.md)
and [ADR 0024](decisions/0024-native-rust-desktop-ui.md) for current evidence.
The [Rust core migration](plans/0023-rust-desktop-migration.md) remains its foundation.

[Project memory and decision service](plans/0022-project-memory-and-decision-service.md)
defines the planned project-scoped agent/team workflow, automatic sourced ADR
drafts, explicit owner acceptance, and local retrieval tools. See
[ADR 0021](decisions/0021-project-scoped-local-decision-memory.md) for the trust
contract and [feature state](state/project-memory.md) for implementation status.

[Scoped agent access](agent-memory-protocol.md) connects owned Codex work and
team roles to current accepted Local ADRs. See [agent state](state/agent-memory.md).

[Automatic drafts](extraction-protocol.md) add project consent, source-version jobs and
attributed proposals through the installed Codex provider. See [draft state](state/automatic-drafts.md).

[Project crew orchestration](state/project-crew-orchestration.md) adds editable
crew recommendations, reusable Project teams, explicit mission starts and
attributed agent communication to the native conversation.

[UI/UX research and design reference](state/ux-design-reference.md) records the
2026-10-25 investigation. [Task0053 implementation](state/ux-rules-implementation.md)
applies the reference to native screens, chips and contextual task actions, with
tests, actual native capture review and a local development package. Component
completion and fixture evidence do not establish measured usability or
accessibility conformance.

[Desktop composition and cleanup](state/desktop-composition-and-cleanup.md)
records the owner's later rejection of the visual composition, the implemented
shell/conversation/inspector changes and removal of obsolete generated builds
and packages. It supersedes the earlier control density; no new package or
installation was produced.

[Codex connection compatibility](state/codex-connection-compatibility.md) fixes
`CODEX_PROTOCOL_MALFORMED` caused by reading full conversation history during
observation. The updated adapter and signed production App have been verified
against the installed Codex 0.163.0; the state records the local App handoff.

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
