# ADR 0021 Project scoped local decision memory

Status: Accepted product and trust contract

Date: 2026-10-14

Implementation: Planned

## Context

The owner requested multiple agents or teams per project, automatic ADR
extraction from project guidance, and a local vector database exposed to agents
through a local service. The owner selected Patchfleet-authored project
messages, tasks, and answers as the default sources. Importing external agent
conversations remains optional. Extracted ADRs start as drafts; only decisions
explicitly accepted by the owner become authoritative.

The current Workspace is one registered Git worktree. A Team selects one
workspace, and provider observation intentionally excludes conversation text.
Neither a worktree path nor a provider session is a sufficient identity for
shared project memory.

## Decision

Introduce a stable local Project above workspaces, agent runs, and teams.
Several worktrees and teams may belong to one project and share its memory.
Separate checkouts are not automatically grouped by display name or remote URL.
Project linkage must be explicit or validated against local repository identity.

Capture owner-authored project inputs through an explicit local ingestion
boundary. Distinguish guidance, requirements, questions, preferences, and
architectural decisions. A message does not necessarily produce an ADR, and an
agent suggestion cannot masquerade as owner acceptance.

Extracted drafts retain source identifiers, source revisions, supporting
excerpts, rationale, extraction version, and optional conflict/supersession
references. Schema validation establishes their shape, not their truth.
Acceptance is an attributable owner command against the current draft revision.
An extractor never promotes a draft on its own, including after a high
confidence score or a phrase it interprets as approval.

Accepted decisions retain immutable text and provenance. A changed decision is
a new proposed record; the previous decision remains authoritative until the
owner accepts its replacement. Supersession is an append-only fact, not an
overwrite. Proposed, rejected, and superseded records remain distinguishable
from active accepted decisions in every agent response.

The existing single local writer owns canonical project, source, and decision
events. The local vector database stores searchable decision text, metadata,
and embeddings as a rebuildable projection. It does not become a second
independent authority for acceptance. ADR Markdown exports are optional local
artifacts and do not automatically modify repository AGENTS.md or ADR files.

Expose a local memory service within the existing Node/Next.js deployable.
Agents access scoped tools through MCP or the same validated service boundary;
they do not open database files or execute arbitrary SQL. The server derives
project scope from the authenticated run binding, not a model-supplied filter.
Agents may read and, when granted that capability, propose drafts. Accepting,
rejecting, or superseding decisions belongs to the owner surface.

Use local embeddings by default. Model identity, revision, dimension, and
normalization belong to the index contract. ADR extraction may use the selected
supported coding-agent provider; local storage does not imply that provider
inference runs offline. No new hosted embedding API or provider credential
copying is introduced by this decision.

Project inputs, ADR contents, excerpts, embeddings, agent search queries, and
memory credentials stay outside default Cloud projections and logs. Existing
Cloud protocol fields are unchanged. A future remote memory capability requires
its own public protocol and authorization decision.

## Consequences

- Project identity and source capture precede vector search and extraction.
- Teams share accepted decisions across worktrees while retaining isolated
  execution attempts.
- Search relevance cannot override acceptance status or project authorization.
- Agents can retrieve exact accepted decisions even when semantic indexing is
  unavailable; degraded search must be labeled.
- Active decisions have a revision that is recorded in task context. Changed
  decisions invalidate stale review context rather than silently rewriting a
  running provider turn.
- Vector engine, embedding model, and MCP packaging are selected through the
  bounded implementation plan and portability checks.

## References

- [Task 0022](../plans/0022-project-memory-and-decision-service.md)
- [ADR 0004](0004-append-only-local-events.md)
- [ADR 0017](0017-use-a-local-workspace-registry-before-cloud-aliases.md)
- [ADR 0019](0019-declarative-packs-and-local-team-orchestration.md)
- [ADR 0020](0020-capability-negotiated-team-protocol-v2.md)
