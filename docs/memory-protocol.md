# Opt-in Cloud memory protocol 1

Status: Accepted implementation contract for Task 0031

Local source/acceptance facts remain canonical. This separate optional protocol
does not change operational V1/V2, pairing, source capture or provider control.
Only explicit owner-selected accepted ADR fields cross this boundary. No source
bodies, origin event records, paths, provider output, drafts or credentials are
publication fields. HTTPS is required except literal loopback development HTTP.
Redirects are forbidden. All replies are JSON/no-store. Unknown fields fail.

## Operations and receipts

`POST /api/memory/v1/hosts/{hostId}/operations` uses the existing paired host
Bearer credential and an exact JSON body (Content-Length required), max192KiB:

`{schemaVersion:1,operationId,hostId,createdAt,expiresAt,payload}`.

operationId is `memory-op:<UUID>`, stable across retries. Times are canonical
millisecond UTC; expiry is after creation, at most seven days later, allowing
offline queued delivery. Fresh operations cannot be more than60seconds in the
future. Local admission commands retain the existing five-minute owner command
window; a queued delivery is a separate durable operation. Authentication,
current billing entitlement, host revocation and project scope are rechecked
even for exact retries. An existing authorized operation returns its original
receipt before expiry/revision checks. Conflicting reuse fails409.
Sharing-off still permits an existing authorized role to recover its exact
earlier receipt (metadata only); it denies new publication/linkage and all
manifest/decision reads. Subscription and revocation checks still apply.

Payload is one of these exact shapes:

- `{type:"create_project",name}`: name1–80 trimmed UTF16 units. New shared ID is
  `shared:<operation UUID>`. Caller becomes its single publisher; revision0.
- `{type:"link_reader",sharedProjectId}`: an explicit reader grant for another
  paired host in the same personal workspace. Existing publisher is not demoted.
- `{type:"publish",sharedProjectId,expectedRevision,decision}`: publisher only,
  sharing enabled. Insert an immutable decision revision and update its manifest
  head atomically. Increment manifest revision by one for each new applied
  operation, including an identical DTO.
  An exact operation retry returns its original receipt without another change.
  If its supersedesDecisionId is a current head, remove that
  predecessor from the manifest in the same transaction. Missing unpublished
  predecessor is allowed; no other project's head can be affected.
- `{type:"sharing",sharedProjectId,expectedRevision,enabled}`: publisher only.
  A changed enabled flag increments manifest revision; disabled projects deny
  new reader/publication access. This does not delete server history or erase
  downloaded copies. Re-enabling never grants new unrelated host roles.

Decision is exactly `{decisionId,revision,status:"accepted",title,context,
decision,rationale,consequences,acceptedAt,sources,supersedesDecisionId}`.
IDs/revisions and text limits match Local V4: title160, decision20000, optional
context/rationale/consequences10000 each; all strings trimmed, safe source-text
controls only. sources is1–8 unique `{sourceId,revision}` opaque local references;
their text cannot be resolved remotely. acceptedAt is the local accepted fact
time. supersedesDecisionId is null or an opaque decision UUID. Cloud does not
claim independently verifying local truth or granting acceptance authority.

All UUID-prefixed IDs use canonical UUIDs. Revisions are safe JSON integers;
expectedRevision may be0, record revision starts1. A receipt is exactly
`{schemaVersion:1,operationId,operationType,requestHash,outcome,reasonCode,
sharedProjectId,revision,role,enabled,completedAt}`. outcome is applied/rejected/
expired; role publisher/reader/null; sharedProjectId and enabled may be null.
requestHash is SHA256 of recursively key-sorted compact JSON of the full operation.
Valid stale/expired operations and semantic publication rejections get durable
terminal receipts. Capacity or immutable-version conflicts must release the
local queue slot; they cannot be transport errors that retry forever. No applied status
is shown locally until a validated matching server receipt is durably recorded.

Transport/auth/subscription/schema/conflicting-identity errors use HTTP status
with `{error:"UPPER_SAFE_CODE"}`; no body or credential appears in errors/logs.
Stable codes include MEMORY_SUBSCRIPTION_REQUIRED(402), MEMORY_ACCESS_DENIED(403),
MEMORY_SHARING_DISABLED(403), MEMORY_CONFLICT(409), MEMORY_INVALID_PAYLOAD(400).
Receipt reason codes include MEMORY_PROJECT_CREATED, MEMORY_READER_LINKED,
MEMORY_PUBLISHED, MEMORY_SHARING_UPDATED, MEMORY_STALE_REVISION,
MEMORY_OPERATION_EXPIRED, MEMORY_CAPACITY_EXCEEDED and MEMORY_CONFLICT.
Conflicting operation identity/hash remains an HTTP409 error, preserving its
original recorded operation rather than creating a new receipt.
Missing/out-of-scope projects deny access uniformly.

## Reads

GET `/api/memory/v1/hosts/{hostId}/projects/{sharedProjectId}/manifest?offset=0&limit=20`
returns exactly `{schemaVersion:1,sharedProjectId,revision,enabled:true,offset,
limit,total,decisions:[{decisionId,revision,title,publishedAt}]}`.
Optional expectedRevision query pins subsequent pages and rejects changes409.
Default offset0/limit20; limit1–20, safe offset, max64 current heads in this first
bounded protocol implementation (technical cap, not a commercial allowance).
Only explicit publisher/reader grants can read. No source or ADR body is in a
manifest. Ordering is decisionId ascending. Readers never overwrite local ADRs.

GET `/api/memory/v1/hosts/{hostId}/projects/{sharedProjectId}/decisions/{decisionId}?revision=N`
returns exactly `{schemaVersion:1,sharedProjectId,manifestRevision,current,
publishedAt,decision}`. Exact immutable published versions remain distinguishable
from current heads; current=false must never be shown as current accepted context.
Each response max192KiB. A complete cached manifest is replaced only after all
its bounded pages/records are consistent; partial download cannot invent a
complete view. Cached content labels last verification/offline/stale status.

Every operation/read checks current paired host, workspace, project role and
server-derived paid entitlement using existing billing authority. Pairing or
payment alone never creates a project/grant/publication. Revocation/off/expiry
blocks new Cloud access; Local and already downloaded data remain available.

No delta feed, organization invitations, multiple publishing writers, publisher
transfer, deletion policy, vector indexing, inference or agent tools is enabled
by this protocol. Periodic bounded manifest polling supplies recovery.
