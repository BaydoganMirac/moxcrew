# Task 0032 Project memory retrieval

Status: Complete for approved scope; deployment/platform release work remains

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

Only current authorized decisions are eligible for results. Recheck access after a search.
Exact reads and publication remain available when search is unavailable. The client labels
keyword fallback and incomplete results.

Use the public result metadata and label fallback explicitly. Test Turkish and English queries
and late matches in long decisions.

Local candidate is the current Rust accepted-decision projection with bounded
ranked keyword retrieval. Measure it against SQLite FTS5 on reproducible
synthetic corpora before selecting a new embedded database. Do not label lexical
search semantic or load a large model into the desktop by default.

## Ownership

- Coordinator: public search contract, ADR, local performance measurements, cross-repository
  HTTP checks and native package.
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

Verify Turkish and English search, current revisions, authorization and revocation, fallback
and unchanged exact reads using synthetic data. Verify that queries stay out of durable
records and operational messages. Measure Local retrieval costs.

## Measured implementation outcome

The final HTTP fixture ranked the expected result in the first five for all 24 synthetic
questions. The Rust desktop kept its existing keyword projection.
