# Task 0031 Explicit paid Cloud memory publication

Status: Approved implementation in progress

Owner approval: 2026-10-15, “Onaylı başla!” after the next-step sequence. Implement
the next concrete milestone: publisher installation A explicitly shares an
accepted ADR; reader installation B sees the identical identity/revision/body.
Local remains free. Vectors, extraction and commercial cutover are later slices.

## Contract and ownership

[Memory protocol1](../memory-protocol.md) is the exact shared contract and
supersedes only the proposed implementation boundary in ADR0030. Preserve V1–V4
meaning. New V5 local publication command/request/fact/receipt and delivery facts
are canonical; no mutable queue becomes a second authority. Frozen outbound
operations bind the paired connection (URL/workspace/host) at admission. A new
pairing cannot silently replay consent to a different account/host.

Local owner actions (existing command envelope, expiry/identity/revision checks):

- `memory_cloud_create {projectId,expectedProjectRevision,name}` queues creation.
- `memory_cloud_link {projectId,expectedProjectRevision,sharedProjectId}` queues
  explicit reader linkage. One mapping per local project/shared project/host.
- `memory_cloud_publish {projectId,decisionId,expectedDecisionRevision,
  expectedSharedRevision,consent:true}` freezes exactly the previewed accepted DTO.
- `memory_cloud_sharing {projectId,expectedSharedRevision,enabled}` queues toggle.
- `memory_cloud_sync {projectId}` explicitly retries delivery/download; background
  sync also resumes it. One unfinished operation per project avoids stale local
  queue races. Bound total pending operations and requests per tick; observe close.
- `memory_cloud_read {projectId,offset?,limit?}` returns scoped cached manifest
  page (10default/20max) plus role/state/revision/freshness, not all body text.
- `memory_cloud_decision_read {projectId,decisionId,revision}` exact cached record.

Mutation receipts use the existing outcome/reasonCode convention; queued means
durable local acceptance, not server publication. No raw credential in events.
Use new `sharedMemory` metadata projection in ordinary snapshots (global revision,
project mapping/role/enabled/sharedRevision, last sync/error and queue status only).
Read results overlay `sharedMemoryPage` / `sharedDecisionDetail` in the Bridge.
Lists invalidate on sharedMemory revision/scope. Exact record DTO/read response
follows protocol. Cache writes are protected/atomic under the same Host owner;
reader data is derived and never enters V4 acceptance facts or operational sync.

Before first HTTP publication attempt, check that the selected local accepted
revision is still current. Record the attempt durably before HTTP. An uncertain
attempt retries the frozen operation to recover a lost acknowledgement even if
local state later changed. Acknowledged local-vs-current divergence is visible;
it never silently republishes new text. No source bodies or origin events leave.

- Local core builder: new `crates/patchfleet-core/src/shared_memory.rs`, necessary
  `lib.rs`/`domain.rs`/`store.rs`/`host.rs`/`cloud.rs`, focused new shared-memory
  tests and narrow persistence legacy projection helper. No desktop/docs/private
  edits. Reuse reqwest/Tokio/SHA and private storage; no desktop DB/service.
- Private Cloud builder: new memory protocol/domain/store/HTTP modules under
  lib, exact routes under app/api/memory/v1, migration0004, focused memory tests,
  disposable PG/HTTP smoke, private task/ADR/state. Narrow existing store exports
  only as needed. No public/desktop files. Memory normalized tables must survive
  existing operational host delete/reinsert saves; do not cascade from host rows.
  Workspace deletion must erase its memory. No production migration/config.
- Native console builder: desktop ui.rs/i18n.rs/bridge.rs only. TR/EN project
  setup, reader link, selected accepted-ADR preview and explicit upload consent,
  queued/published/conflict/access/subscription state, current vs historical
  downloaded reads. Disable stale/busy controls; preserve retry/editor safeguards.
- Coordinator: public contract/ADRs/plans/state, two-repo integration/proofs,
  main/smoke fixture, version/package and review. Independent reviewer after stable
  commits, no permanent workers or live provider/billing actions.

## Acceptance and exclusions

Real two-Host -> HTTP -> disposable PostgreSQL -> reader exact read. Test duplicate
and changed op identity, expiry, all local crash prefixes, lost acknowledgement,
offline/restart, stale local/Cloud revisions, host/workspace/project isolation,
revoked host, sharing off and inactive entitlement, partial/malformed/oversized
responses, canonical source exclusion and unchanged V1/V2. Use no deployed DB or
environment credentials; fixture starts an isolated local PostgreSQL cluster or
container and destroys only its own resources. Run existing core/native/private
tests and checks, then independent review and native development package.

No price/quota offer/trial or existing-customer cutover, organization invites,
publisher transfer, live deployment/migration, pgvector/inference/provider calls,
source transcript import, installed-App replacement or owner-data modification.
