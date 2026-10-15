# Task 0029 Decision memory and subscription Cloud publication

Status: Proposed implementation plan — runtime not started

Owner request: 2026-10-15, turn the selected product model into an implementation
sequence. The commercial boundary is accepted; this card proposes the remaining
technical choices and bounded slices, not a deployment or payment change.

## First end-to-end outcome

On installation A, an owner creates a sourced ADR, accepts it and explicitly
publishes it to an authorized subscription Cloud project. Installation B links
to that project and reads the identical decision identity, revision, status and
published manifest. Restart, duplicate delivery and network loss preserve that
outcome. Local-only projects keep working without an account or subscription.

Use [ADR 0030](../decisions/0030-shared-decision-publication.md)'s proposed
single-publisher rule. The first release supports one owner's paired devices;
it adds no organization invitations, billing seats or generic multi-writer merge.

## 1. Local decisions and native UI

Extend existing project/source storage and the single Host with versioned
decision commands/facts: propose, accept, reject, supersede, scoped list and
exact read. Decision fields include identity, project, revision, title, context,
decision, rationale, consequences and source ID/revision references. Acceptance
freezes content and provenance; a source edit cannot rewrite an accepted ADR.
A replacement becomes authoritative only through an explicit owner command.

Add a Turkish/English Decisions view with status, source references and history.
Preserve existing V1/V2/V3 meaning, exact retry receipts, expiry, revision checks,
replay and metadata-only operational snapshots. Use bounded scoped text reads.
Reuse the current log/projection first; local SQL/vector packaging is not a
prerequisite for this slice. Local semantic retrieval remains a later free
capability with its own embedded-engine/model measurements.

Ownership for the bounded card: core builder owns `project_memory.rs`, necessary
`domain.rs`/`store.rs` validation and focused memory tests; console builder owns
`ui.rs`/`i18n.rs`; coordinator owns Host/bridge integration, shared contracts and
docs. Assign exact file boundaries before concurrent work. Reviewer starts on
a stable implementation commit.

Exit: real owner UI -> accepted ADR -> restart -> identical exact read. Tests
cover crash prefixes, duplicate commands, stale acceptance, cross-project IDs,
immutable source provenance and supersession.

## 2. Cloud publication and desktop sync

Specify memory protocol version 1 separately from operational V1/V2. Define
project creation/linking, writer/reader roles, publish, operation receipt,
paginated changes, manifest and exact decision reads. Keep strict schemas,
request/page limits, idempotent operation IDs, expected shared revision and
payload fingerprints. Oversize text must produce an explicit error, never
silent truncation. Existing pairing alone grants no memory access.

The Cloud builder owns its separately approved private storage/API/auth tests.
The public coordinator owns the reduced protocol contract; core owns durable
outbox/cache/receipt handling; console owns preview/consent and sync states.
Keep private SQL, commercial implementation and infrastructure out of public docs.

Use the existing Cloud PostgreSQL/auth/backend. New memory services require
verified paid entitlement in server checks, not a desktop toggle or checkout
return. Existing billing state names may remain internal during this feature;
the all-Cloud commercial cutover belongs to slice 6.

Locally persist publication intent before HTTPS, record acknowledgement through
the writer, and resume after restart. Readers use a separate protected derived
cache, report revision/freshness and cannot mutate the publisher's acceptance
facts. UI distinguishes local-only, queued, published, stale, conflict and
access/subscription problems. Revocation and sharing-off block new shared reads
and writes; server deletion and offline-copy retention are separate behaviors.

Exit: the two-installation outcome above through real disposable HTTP/PostgreSQL
fixtures. Prove tenant/project/host-role denial, duplicate success, changed-payload
denial, lost acknowledgements, stale revisions, revoked access, inactive
subscription, offline queue recovery and no forbidden operational payloads.
Only disposable integration databases may be initialized by the test card;
production migrations and deployment remain separate owner actions.

## 3. Cloud vectors and local retrieval evaluation

Add pgvector to the Cloud test/package matrix. Select a multilingual embedding
model only after Turkish/English fixtures, cost, license and model/version/
dimension/normalization checks. Persist indexing jobs with deduplication,
leases, bounded retries and a visible cursor. Store vector rows against exact
published decision revisions; reject model-space mixing.

Start with exact vector retrieval and project/status filters. Add approximate
indexes only when measured corpus size/latency warrants them. Measure recall,
latency and cost before setting commercial allowances. Keyword fallback and
index lag must be labeled. Index failures cannot fail an acknowledged publication
or break exact reads. In parallel, evaluate the lightweight embedded Local
search option without requiring Local users to subscribe.

Exit: reproducible TR/EN retrieval, project isolation, inactive/superseded
exclusion, rebuild, model change and embedding-outage checks.

## 4. Scoped agent access

Add the bounded local MCP bridge to the same Host service: `project_context`,
`memory_search`, `decision_read`, then optional `decision_propose`. Bind scope,
permissions, expiry and revocation to the actual run. A model-supplied project
ID cannot select another project. Record the manifest revision used by work;
changed decisions make context stale rather than rewriting an active turn.

Exit: real provider handshake and two worktrees/agent roles reading the same
accepted manifest, with forged scope and completed/revoked run access denied.

## 5. Automatic sourced drafts

Capture a durable extraction job per project/source ID/revision/extractor
version. Use bounded supported provider execution with proven structured output
and no workspace writes. Extraction runs against local sources and retrieved
decisions; enabling Cloud sync does not upload all source conversations.
Validate provenance, source-edit races, no-ADR cases, duplicates and conflicts.
Only owner acceptance activates a draft; only explicit selection publishes it.

Exit: a real project message produces a sourced draft; questions can produce no
ADR; provider failure preserves source/job state and never invents acceptance.

## 6. Commercial cutover and release

Resolve price, billing unit, allowances, any trial, existing-customer transition,
cancellation/export and retention. Update pricing/onboarding and verified
server-side Cloud access together. Keep billing/account management and terminal
receipts usable as required for recovery. Expiry never disables Local, stops
its agents or removes downloaded records. Keep actual model-provider costs
separate and do not promise unlimited hosted inference.

Use measured costs from slice 3 and explicit operator configuration. Test
activation, duplicate/out-of-order billing events, failed payment, cancellation,
renewal and downgrade before an owner-operated launch. No billing-provider
call, production migration, deployment or publication is part of this plan.

## Delivery discipline

Each slice needs its own concrete approved implementation card, tests, state
update and reviewable commit. Slices 1-2 form the first end-to-end milestone;
vectors and automatic extraction follow working exact shared retrieval.
Current tests prove the existing product only; no new slice is claimed as
implemented or verified by writing this plan.
