# Task 0030 Local sourced decisions

Status: Implemented — independent acceptance review accepted

Owner approval: 2026-10-15, “Onaylı başla!” following Task 0029's first code
slice: native Decisions view and durable local ADR lifecycle. Local remains
free, with no account, subscription, inference or embedding prerequisite.

## Outcome

An owner selects existing project sources, writes an ADR draft, explicitly
accepts or rejects it, and can replace an accepted ADR with another draft.
Restart preserves exact content, cited source versions, history and status.
Ship a reviewable native development package. This card implements slice 1 of
Task 0029, not Cloud publication, vectors, MCP or automatic extraction.

## Contract

Use the existing Host/single writer and new V4 decision events, as specified by
[ADR 0031](../decisions/0031-versioned-local-decision-events.md). Existing
V1/V2/V3 meanings and Cloud operational schemas stay unchanged. Mutations
carry the existing commandId/createdAt/expiresAt envelope, owner attribution,
durable terminal receipts, exact retry identity and prefix recovery.

- `project_decision_propose`: projectId, expectedProjectRevision (current V3
  source/project revision), title, context, decision, rationale, consequences,
  sources: 1–8 unique `{sourceId, revision}` references. Sources must belong to
  the same project and match current revisions. Title is required, max160 UTF16
  units; decision required, max20000; optional context/rationale/consequences
  max10000 each. Text follows source control-character rules. Host intake is
  bounded at192KiB. Body is immutable; a changed draft is a new record.
- `project_decision_accept` / `project_decision_reject`: projectId, decisionId,
  expectedDecisionRevision. Only a proposed record can transition. Acceptance
  also requires current cited source revisions; rejection remains available
  after a source edit.
- `project_decision_supersede`: projectId, decisionId (accepted predecessor),
  expectedDecisionRevision, replacementDecisionId (proposed replacement),
  expectedReplacementRevision. Atomically accepts the replacement and
  supersedes the predecessor, with reciprocal relations. Validate replacement
  sources before applying; predecessor remains active on any rejection.
- `project_decisions_read`: projectId, optional offset (default0), limit
  (default10, max20), query (max160), status (`all`, `proposed`, `accepted`,
  `rejected`, `superseded`; defaultall). Ephemeral scoped read returns
  `{reasonCode:"PROJECT_DECISIONS_READ",projectId,revision,offset,limit,query,
  status,total,decisions:[summary]}`. Summary has decisionId/projectId/title/
  status/revision/createdAt/updatedAt/sourceCount/supersedesDecisionId/
  supersededByDecisionId. Query matches ADR text, never escapes project scope.
- `project_decision_read`: projectId, decisionId. Ephemeral exact read returns
  `{reasonCode:"PROJECT_DECISION_READ",projectId,revision,decision,history,
  sources}`. Decision includes summary fields plus context/decision/rationale/
  consequences, actorId and cited sources. History is bounded lifecycle facts
  with status/revision/actorId/at. Sources resolve original cited source records,
  including original title/text/kind/revision/origin, even after later edits.

Decision IDs are `decision:<uuid>`; propose derives UUID from command identity.
Decision revision starts1; each transition increments it. Per-project decision
revision and global decision revision increment once per applied command;
supersession touches two records in one fact. Source/project revisions do not
change on a decision command. Receipts include schemaVersion4, commandId,
action, outcome, reasonCode, completedAt, projectId, projectRevision (decision
manifest revision), decisionId, decisionRevision, replacementDecisionId and
replacementRevision (null/0 when absent).

Full `projectDecisions` cache (`project-decisions.json`) is rebuildable. Normal
snapshots expose only `{schemaVersion:4,revision,projects:[{projectId,revision,
decisionCount,acceptedCount,proposedCount}]}`. Bridge overlays bounded
`decisionPage` and `decisionDetail` only for explicit reads, invalidated on
project decision revision changes. Existing `memoryPage` supplies source
selection and is invalidated by V3 revision. Reads/queries produce no events.

## Ownership and acceptance

- Core builder: new `crates/patchfleet-core/src/decisions.rs`, `lib.rs`, necessary
  V4 validation/projection integration in `domain.rs`/`store.rs`, focused
  `tests/decisions.rs`, plus the narrowly granted `tests/persistence.rs` legacy
  projection helper. Do not edit Host, desktop, other shared contracts or
  existing tests without coordination. No dependency/framework additions.
- Console builder: desktop `src/ui.rs` and `src/i18n.rs` only. Dedicated
  Turkish/English Decisions view with project/filter/list/detail, readable
  status/history/source evidence, draft creation and explicit lifecycle actions.
  Preserve owner text, bounded reads, busy/stale guards and folder onboarding.
- Coordinator: Host/action boundary, Bridge, smoke fixture, Host integration
  tests, version/configuration, docs/state, integration and package.
- Independent reviewer: read-only stable implementation commit after checks.

Tests cover exact retry/changed identity, expiry, every crash prefix, replay,
cross-project denial, source edit before/after acceptance, forged facts,
atomic supersession, bounded ephemeral reads and operational Cloud exclusion.
Native checks cover actual click payloads and TR/EN/minimum window rendering.
Run Rust tests, fmt, strict Clippy and existing legacy parity. Inspect the real
native render, build a production package with smoke hooks disabled, verify
signature/image integrity, document upgrade/rollback and remaining limits.

No owner data, installed app, provider settings/transcripts, private Cloud,
production database, payment provider, deployment or publication is modified.
Workers commit only their owned files locally and report checks/risks. Root
updates feature state and reviews the final staged file set.

## Integration evidence

- Core `9ab17bb`, Host/Bridge `95ab7c9` and native UI `6aa36d7` implement the
  agreed boundaries without new dependencies or private Cloud changes.
- 81 core and 42 desktop tests pass, including all four lifecycle prefix
  recovery paths, immutable edited-source evidence, stale/foreign denial,
  atomic replacement, exact retries, expiry/clock corrections, maximum CJK
  intake, localized owner clicks and non-overlapping minimum navigation.
  One existing manual large-log probe is ignored. All 149 legacy tests and
  246 V1 prefix hashes pass; fmt and strict workspace Clippy pass.
- Independent core/Host/Bridge review accepted `9ab17bb`/`95ab7c9`. Native review accepted
  `6aa36d7` and the recovery fixes through `d652bf9`; no production data/provider was used.
- Review recovery fixes `f4bc387`/`5b88067`/`d652bf9` prevent replayed old
  successes from clearing new draft edits and keep pending owner command/error
  state ahead of automatic reads. An actual projection-cache failure regression
  proves exact retry without new canonical bytes; UI tests cover edit/citation
  preservation, no consumed read keys and localized recovery controls.
- Actual TR/EN native renders and normal close pass with disposable ADR data.
  The production 0.6.0 App/DMG passes ad hoc signature, version, five folder
  purpose keys, absence of test hooks/browser runtime and image integrity.
  NSWorkspace returns the actual test bundle/PID/window; normal close releases
  the writer. Final production OS pixels remain unavailable, explicitly distinct
  from the inspected GL captures. See [native state](../state/native-rust-desktop.md)
  for artifacts, checksums, resource sample and platform limits.
- Local is free and offline. Cloud publication, pgvector/embedded retrieval,
  MCP, automatic extraction and commercial entitlement cutover remain separate
  Task 0029 slices. No installed application or owner data was changed.

Final independent review accepted the code through `d652bf9` and the corrected
state/install/release declarations. No concrete blocker remains. The reviewer
read the artifact evidence; the coordinator ran the package checks. Native
Windows/Linux execution, public signing and final production OS pixel capture
remain explicitly unverified rather than implied by the configured CI matrix.
