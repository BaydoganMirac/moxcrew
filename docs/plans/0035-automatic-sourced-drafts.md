# Task 0035 Automatic sourced ADR drafts

Status: Complete; independently reviewed and locally verified

Owner approval: 2026-10-15, “Sıradaki işe geçelim”, following the delivered
Task0034 handoff that named automatic source-backed ADR drafting as next.

## Outcome

An owner enables automatic drafting for a Local project. New Patchfleet messages,
owner tasks, team goals, owner answers and notes produce bounded, source-backed
ADR proposals through the installed Codex provider. Existing sources can be
explicitly queued one at a time. Nothing becomes accepted or replaces a rule
without the existing explicit owner command. Local remains free; provider
inference uses the owner's existing provider access and may consume its usage.
The desktop adds no model, database, browser runtime or API-key setup.

## Contract

- Default off per project. A durable owner configure command requires explicit
  provider-disclosure consent when enabling. Only source creation/edits after
  that enable fact are automatically eligible; enable never silently backfills
  history. The selected existing-source enqueue action is deliberate.
- V7 extraction command/settings/job facts extend the existing writer. Keep
  V1–V6, V4 acceptance and Cloud allowlists unchanged. Metadata projections are
  rebuilt in memory; no second authority or source-body copy is introduced.
- Job identity derives from project/source ID/source revision/extractor version.
  At most32 queued/running jobs globally, one active provider process, at most
  three explicit attempts. Failed/interrupted jobs need an owner retry; no
  automatic paid retry loop. Restart marks interrupted jobs, retaining sources.
- Queue only current owner sources in the frozen project. Generated role
  contracts and provider transcripts remain excluded. Source edit or disabling
  policy invalidates pending/running work; no stale completion can publish.
- One source per job, full source text within existing50000 UTF16 limits.
  Core input is bounded at240KiB, final provider prompt at256KiB; relevant Local decision context is bounded and
  explicitly labeled partial if omitted for size. No source is silently cut.
- Installed Codex0.155.1+ uses supported exec JSONL/output-schema, ephemeral
  sessions, read-only sandbox, ignored user config/rules, disabled tools/apps/
  hooks/plugins/MCP and an isolated scratch cwd. No coding worktree is supplied.
  Prompt travels on stdin, never argv. Native/provider output stays inside the
  adapter. Bound60second timeout, cancellation, output and process-tree cleanup.
  Inference runs outside the render thread and operations mutex.
- Structured output is exactly {proposals:[...]}, max2. Each proposal contains
  title/context/decision/rationale/consequences, evidence:[{sourceId,revision,
  excerpt}], relation:new|possible_conflict|possible_duplicate, relatedDecisionIds.
  Evidence must cite the queued current source with an exact nonempty substring,
  max1000 UTF16. Related IDs must be current accepted decisions in this project.
  Empty proposals is a valid no-ADR result. Unknown fields, foreign IDs,
  unanchored excerpts and malformed/oversize output fail the whole result.
- Completion validates source/settings/job attempt and publishes existing V4
  proposals plus V7 generator provenance in one writer transaction. Existing V4
  local-owner denotes the delegating command authority, never a claim that the
  model wrote human text. Native detail explicitly identifies Codex-generated
  drafts, extractor version, evidence and possible relations. Exact normalized
  decision text duplicates across all statuses are suppressed; source/version
  job retries cannot duplicate an acknowledged proposal. No auto acceptance,
  rejection, supersession, publication or execution tool is added.
- Snapshot metadata contains counts, state/reason codes and opaque references,
  no raw input/output/quotes. Explicit Local detail may resolve provenance.
  Source/model output remains outside default operational Cloud exports. The
  three agent tools still expose only accepted ADRs.

## Ownership

- Coordinator: docs/ADRs/state, lib.rs/domain.rs/store.rs/host.rs, new
  host_extraction.rs + host_extraction_tests.rs, Bridge and main smoke wiring,
  release/version/configuration and real Host/native integration.
- Core builder: new extraction.rs + tests/extraction.rs only. Own V7 ledger,
  source eligibility, lifecycle/dedup/provenance and transactional V4 proposal
  reuse. Do not edit shared schema, Host, provider or native files.
- Provider builder: providers.rs + tests/provider_extraction.rs only. Own bounded
  supported Codex extraction process and native protocol parsing. Existing work
  controller and memory lifecycle behavior must stay compatible.
- Console builder: desktop ui.rs + i18n.rs only. Own consent/enable/disable,
  explicit existing-source enqueue, status/manual retry and clear generated
  draft provenance in TR/EN. No storage or provider writes.
- Independent reviewer: stable commits, read-only; authority/provenance/replay,
  cancellation/consent/source races, bounds, provider containment, Cloud canaries
  and UI command truthfulness.

## Acceptance

Real Host with bounded native protocol fixtures: new TR/EN source -> sourced
proposed ADR -> explicit owner acceptance -> same accepted-only MCP behavior.
Cover no-ADR questions, duplicate runs/content, malformed/foreign/unanchored
output, source edit/disable during inference, missing/unsupported provider,
timeout/cancel/restart, failed manual retry, command idempotency and crash
prefixes, quote/generator preservation after source edits, no auto supersession,
default-off/no-backfill, responsive Local commands and default Cloud canaries.
Verify installed CLI flags/schema without reading credentials or owner config.
Real model quality is separate from deterministic fixture proof and must be
reported honestly. Run affected/full Rust checks and legacy parity, independent
review, actual native frames and a feature-off macOS development package.
No owner data, Applications install, global provider config, private Cloud,
publication, deployment or real inference turn is modified/run by development
without a separately warranted explicit verification step.

## Handoff

Implementation and review evidence: [automatic draft state](../state/automatic-drafts.md).
Core8d4c953, provider0e4ca57/899b0c5 and consolef571103/4589b31 are locally
committed; coordinator integration includes native Host lifecycle, bounded
source-job recovery and0.11 packaging. No inference turn, private Cloud change,
Applications replacement, publication or deployment was performed. Real model
quality and Windows/Linux native runs remain explicit verification limits.
