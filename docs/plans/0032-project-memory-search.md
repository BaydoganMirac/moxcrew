# Task 0032 Project memory retrieval

Status: Implemented and measured; final independent integration review in progress

Owner approval: 2026-10-15, “Onaylı başla” after the semantic search sequence.
Implement the next bounded milestone: natural-language TR/EN queries retrieve
the correct currently published ADR in an explicitly selected paid Cloud
project. Keep Local free and measure its lightweight retrieval option.
No deployment, production migration, billing change, agent tools or automatic
extraction is included.

## Contract

Extend the separate Memory protocol1 with an explicit, non-persisted search
request. No V1–V5 event meaning changes. Owner queries cross the Cloud boundary
only on an explicit Cloud search action; no automatic searching while typing.
Never include Local source bodies, history, paths or credentials in that request.
Searches create no canonical events and are excluded from logs/snapshots.

Cloud uses exact cosine retrieval in PostgreSQL/pgvector. Published ADRs queue
deduplicated indexing work transactionally; inference is outside the publication
transaction. Leased jobs have bounded attempts, stale-worker fencing and restart
recovery. Current heads and the active model space are filtered on every query.
Revocation, sharing-off, project grants and paid entitlement are checked before
and after inference. Indexing/query failure cannot break publication or exact
reads. Partial indexes and unavailable inference use an explicitly labeled
keyword fallback over current authorized decisions.

Evaluate multilingual-e5-small q8, 384 dimensions, mean pooling and L2
normalization, with immutable artifact revision and query/passage prefixes.
Cloud inference runs from explicitly prepared local model files; no hosted
model API, provider credential, automatic runtime download or desktop model
dependency is introduced. Accept this candidate only after actual TR/EN
paraphrase/cross-language and late-chunk checks. Model identity, revision,
dimension, normalization and chunk version form a single immutable model space.
Tokenizer-bounded overlapping chunks must cover full accepted text.

Local candidate is the current Rust accepted-decision projection with bounded
ranked keyword retrieval. Measure it against SQLite FTS5 on reproducible
synthetic corpora before selecting a new embedded database. Do not label lexical
search semantic or load a large model into the desktop by default.

## Ownership

- Coordinator: reduced public search contract, ADR, docs/state, model dependency
  and encoder/worker/preparation modules in private Cloud, disposable pgvector16
  setup and cross-repo proofs, model/Local measurements and native package.
- Cloud builder: new lib/memory-search.mjs, lib/memory-index.mjs,
  lib/memory-search-http.mjs, new search route, migration0005, narrow publish
  queue integration in lib/memory.mjs, focused tests and indexing CLI. No encoder,
  public, desktop or dependency edits. Reuse memoryTransaction/memoryProject.
- Rust builder: new retrieval module and focused tests, narrow Host/Store/Cloud
  integration and lib export. Local accepted-only keyword search plus explicit
  Cloud search read; shutdown cancellation and existing safe HTTP boundary.
- Native builder: ui.rs/i18n.rs/bridge.rs only. Explicit query, scope and mode,
  clear Cloud-query disclosure, result/fallback/index-state display, exact read
  action and stale/busy/error recovery. No automatic searches or Local authority
  changes. Coordinator owns main.rs smoke.
- Independent reviewer after stable commits: scope, current heads, paid/auth,
  query privacy, lease fencing, model space, bounds and UI retry/scope behavior.

## Acceptance

Real pinned model inference and PostgreSQL16/pgvector, TR/EN paraphrase and
cross-language fixtures, full-text chunk coverage, stale/replacement exclusion,
tenant/project/role/revocation/subscription/sharing denials, model mismatch,
lease crash/reclaim/stale completion, outage keyword fallback, rebuild and
unchanged exact-read/publication behavior. Prove queries do not enter durable
records or operational payloads. Measure candidate recall, cold/warm latency,
RSS/model size and Local retrieval costs; do not invent commercial allowances.
Use only synthetic data and fresh disposable databases. Finish with affected
checks, independent review and a feature-off native development package.

## Measured implementation outcome

The initially evaluated E5-small q8/FP32 and MiniLM candidates missed bilingual
quality. MPNet q8 was selected after the same fixed topical questions: final
13-document real worker/HTTP/SQL fixture21/24 Top1,23/24 Top3,24/24 Top5, plus
late-text rank1 and maximum-field CJK coverage. Protocol/ADR0033 pin768 dimensions,
no prefixes and112/24-token chunks. Free Local keeps the measured Rust projection;
no database/model dependency was added to desktop. See the search state for proof
and remaining deployment/platform boundaries.
