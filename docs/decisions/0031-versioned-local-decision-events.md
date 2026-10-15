# ADR 0031 Versioned local decision events

Status: Accepted for Task 0030

Date: 2026-10-15

## Decision

Implement the accepted ADR 0021 trust contract as V4 commands/facts/receipts in
the existing local log. One writer owns proposed, accepted, rejected and
superseded records. Draft content is immutable in the first slice; edits create
a new draft. Only explicit owner commands change authority. Supersession accepts
a proposed replacement and supersedes its accepted predecessor in one fact.

Freeze source ID/revision and provenance on proposal, validate source freshness
again on acceptance, and resolve original cited text from canonical source
history. Later source edits cannot rewrite an accepted ADR. Do not duplicate
whole source bodies in every decision event. Decision revisions and their
per-project manifest revision are separate from V3 source revisions.

Keep full decision text in a protected rebuildable projection. Runtime snapshots
carry counts/revisions only; bounded scoped list and exact reads are ephemeral.
Existing Cloud V1/V2 allowlists gain no decision or source fields. This slice
adds no Cloud upload, vector engine, inference, MCP or extraction scheduler.

## Consequences

Local decisions work offline and remain free. Exact reads and history precede
search indexing. The future Cloud publication contract can refer to stable
decision identities and revisions without becoming local acceptance authority.
Automatic drafts can later use the same provenance validation, with their own
extractor attribution contract; they cannot invoke owner acceptance.

Older releases do not understand V4 events. Export/back up the whole data
directory while the app is closed before upgrading; rolling back requires the
pre-upgrade data copy. No existing owner log is touched by development tests.

See [Task 0030](../plans/0030-local-decision-lifecycle.md) for the bounded API,
ownership and acceptance checks. This does not accept the proposed Cloud
publication protocol in ADR 0030.
