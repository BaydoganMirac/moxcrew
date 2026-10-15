# Local automatic ADR drafts

Task0035 / native0.11 / local event schema7. This is a Local extension; the
operational Cloud V1/V2 and separate Memory protocol1 contracts are unchanged.
[ADR0036](decisions/0036-consented-provider-draft-extraction.md) records the choice.

## Owner commands

All three use the existing commandId/createdAt/expiresAt envelope, canonical
writer, requested fact and terminal receipt. Duplicate command IDs return their
original receipt. Unknown fields and stale expected revisions are rejected.

| Action | Fields besides envelope |
|---|---|
| project_extraction_configure | projectId, expectedPolicyRevision, enabled, consent |
| project_extraction_enqueue | projectId, sourceId, expectedSourceRevision |
| project_extraction_retry | projectId, jobId, expectedAttempt |

Absent settings mean disabled with policyRevision0. Enabling requires consent;
only subsequent source creation/edits are automatically eligible. Old sources
must be queued explicitly while enabled. Sources are current owner messages,
task instructions, team goals, answers and approval notes in that project.
Provider transcripts and generated role contracts are excluded. Disable cancels
pending/running jobs; enabling again does not revive them automatically.

Identity hashes project/source/revision/extractorVersion. At most32 queued or
running jobs exist globally, one inference runs at a time, and at most three
explicit attempts are allowed. Queued jobs have attempt0. Failure, interruption
or cancellation requires an explicit current-attempt retry; changing a source
requires processing its new version. Terminal states never cause paid retry
loops. A full queue leaves eligibility visible to later reconciliation.

## Durable facts and recovery

V7 types are extraction.command.requested, extraction.command.receipted,
extraction.settings_changed, extraction.job_queued, extraction.job_retried,
extraction.job_started, extraction.job_finished and extraction.proposal_bound.
The canonical log owns these facts; no second database or job authority exists.

Before a V4 proposal command, a V7 proposal_bound records deterministic decision
and command identities, job/attempt/index, generator, source version, exact
excerpt, relation and related IDs. Each readable generated draft is attributable
even after a torn completion prefix. Existing V4 recovery runs before V7
recovery. An interrupted completion never silently resumes inference. V4 still
uses local-owner as the command's delegated authority; the exact Local view
explicitly labels the Codex generator rather than claiming human authorship.
Acceptance, rejection, supersession and publication retain their existing owner
commands. Source freshness is rechecked before completion and acceptance.

Successful empty output yields completed/EXTRACTION_NO_ADR. Exact normalized
text duplicates across every ADR status yield no extra proposal. Evidence and
generator remain immutable after later source edits. Relations are advisory;
possible conflicts never replace an accepted decision.

## Provider boundary

The installed Codex CLI must support0.155.1+ exec JSONL and output-schema.
The adapter ignores user config/rules, uses an ephemeral read-only session in a
protected UUID scratch directory, disables tools, shell, MCP, plugins, hooks,
apps and skill discovery, and supplies no worktree cwd. Source text travels on
stdin. The selected/new source and relevant accepted decisions (including older ADRs)
are sent as provider context. The current provider account may send this data to its service and use
its quota; this is separately disclosed before enablement. No new API key,
model download, database daemon or production dependency is added.

The full single source is preserved up to the existing50000 UTF16 limit.
Serialized core input is bounded at240KiB, final prompt at256KiB. Accepted Local
context is bounded to20 records and size; contextPartial states omitted context.
Output is exactly {proposals:[...]}, at most two records, each with title,
context, decision, rationale, consequences, evidence, relation and
relatedDecisionIds. Evidence contains exactly one current sourceId/revision and
an exact nonempty excerpt of at most1000 UTF16 units. Relation is new,
possible_conflict or possible_duplicate; at most8 unique current accepted
same-project related IDs are allowed. New requires no related IDs; other
relations require related IDs. Unknown fields, invalid references/quotes or an
invalid member reject the entire batch before any proposals are written.

Bound60seconds includes probe and execution; stream lines<=256KiB,
stdout<=1MiB and final JSON<=128KiB. Cancellation kills/reaps the process tree;
source edit, policy disable and shutdown invalidate pending completion. Scratch
files are removed after cleanup and own UUID leftovers are removed on reopen.
Provider stderr/output never becomes a notice or metadata field. Safe reason
codes distinguish unavailable/unsupported, timeout, invalid output, provider
failure, interruption and cancellation. Local owner commands never wait for
model completion or hold their operations mutex across inference.

## Read and upload boundaries

Snapshot projectExtraction contains schema/revision and project policy state,
status counts and the most recently queued10 jobs; UI identifies this partial
list. Paged Local source reads add only the current source-version job metadata
and extractionRevision, so older failed jobs remain reachable without enlarging
the snapshot. Pages invalidate on V7 change. It contains no prompt, output or excerpt. Exact Local decision reads may
resolve immutable generator/evidence. Default Cloud exports omit all V7 state
and text. The three agent MCP tools continue to expose current accepted ADRs
only; no model write tool or additional acceptance authority exists.

Native0.11 is required after a V7 write. Stop the app and preserve a pre-upgrade
backup for rollback. Opening old compatible state does not enable extraction.
