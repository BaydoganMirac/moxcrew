# ADR 0030 Shared decision publication through one publishing host

Status: Proposed — implementation design, not an enabled capability

Date: 2026-10-15

## Context

ADR 0029 selects free Local and subscription Cloud. Local project/source memory
exists, but decision lifecycle, shared publication, agent tools and embeddings
do not. The first useful shared-memory outcome is two installations reading
the same owner-accepted ADR revision.

## Proposed first-release boundary

Keep local acceptance and source evidence canonical in the existing single
Rust writer. A Cloud-enabled project explicitly selects one paired publishing
host. Other installations explicitly link as readers. A reader can keep local
private sources/drafts, but cannot accept or publish a change to shared memory.
Cloud/browser authoring and publisher transfer require later contracts; they
are not implied by ordinary pairing.

The publishing host writes its accepted-decision revision and publication intent
durably before sending. Cloud atomically validates and stores the selected
publication, its operation receipt and feed revision. The local writer records
the acknowledgement separately. An accepted local decision is not shown as
published until this acknowledgement exists. Offline publication stays queued.

Cloud's published manifest is the common read reference for authorized clients;
it is a projection of explicitly published owner facts, not a second acceptance
writer or execution authority. Reader caches never overwrite owner acceptance
events. Conditional revisions reject stale publication; conflicting operation
reuse fails rather than overwriting records.

## Data and access

- Project sharing and reader mapping are explicit. Never infer shared identity
  from folder names, paths or Git remotes.
- Introduce a separately versioned memory protocol. V1/V2 operational sync and
  their allowlists remain unchanged. Payment does not enable data publication.
- Publish only previewed, selected ADR fields and opaque provenance references.
  Raw source messages, tasks, answers, transcripts and credentials stay local.
  A reader must show when a source reference cannot be resolved on its device.
- Authorize workspace, project, host role and paid-service access on every
  Cloud operation. Agent tools additionally derive scope from their run binding.
  Agents never receive database credentials or acceptance authority.
- Disabling sharing stops future publication and revokes shared access. It is
  distinct from deleting server records or erasing already-downloaded copies;
  those user-visible operations and retention policy need their own scope.

## Storage and failure behavior

Cloud PostgreSQL stores publications and immutable revisions. pgvector and
embedding jobs are derived from those published revisions. Exact reads and the
accepted-decision manifest must work without an embedding worker or vector
index. Rebuilding an index must not change acceptance or publication history.

Local operation, downloaded decisions and owner sources stay available during
outages or subscription expiry. Cached shared reads include their revision and
freshness. Expiry stops new paid work without inventing successful sync or
discarding already-acknowledged results. The first release does not claim
offline semantic search without a local embedding model.

## Consequences

One publisher avoids multi-writer merge semantics in the first release. If that
host is offline, readers can use published/cached decisions, while new shared
acceptance/publication waits. Simultaneous shared editing, publisher recovery
or transfer and multi-user collaboration are deliberate later work.

See [implementation plan](../plans/0029-decision-memory-and-cloud-publication.md),
[memory contract](0021-project-scoped-local-decision-memory.md) and
[commercial boundary](0029-free-local-and-subscription-cloud.md).
