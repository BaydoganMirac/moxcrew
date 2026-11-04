# Dated source change tracking

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0078](../plans/0078-dated-source-change-tracking.md)

**Project tools → Create outputs → Track source changes** selects an earlier and
later frozen Project resource, a report title and two ISO capture dates. The form
adds a typed request to Chat; it does not fetch a live page, send the message,
prepare a plan or start work.

The local `research.changes` publication boundary extracts supported text, binds
both exact source and extracted-text hashes, removes blank lines, normalizes
whitespace and compares up to 2,000 meaningful lines. It requires the later date
to be strictly newer and caps the review ledger at 1,000 changes.

One checked request produces:

- `change-report.md`, a dated summary and before/after line review;
- `change-report.json`, with exact hashes, dates, counts and structured changes.

Identical normalized text remains a successful, explicit no-change result.
Formatting-only byte differences set `ignoredFormattingOnly: true`; Mox does not
turn them into false product changes.

## Verification

- Focused core coverage compares a changed Markdown pair, proves both input
  SHA-256 values, dated line-bound modifications/additions and privacy of the
  result. It also proves formatting-only no-change output and rejects reversed
  dates.
- Focused desktop coverage proves two supported resources, one excluded binary,
  strictly ordered dates, visible 800×560 primary action, staged typed request
  and zero direct Host commands.
- Retained native evidence: [Turkish dark source-change form](../screenshots/source-change/source-change-tr-dark.png),
  SHA-256 `625f6329b90e0a233be92a6bf45cd23a2e6cd545f1d206b56749e9ab921bfb30`.
- The full workspace passes with 557 tests and 6 explicit fixture/manual
  ignores when run with one test thread: `patchfleet-core`'s library has 118
  passing tests with 3 ignores and the desktop has 253 passing tests. The one
  timing-sensitive child-process cleanup test also passes alone. Strict Clippy,
  formatting, screenshot checksum and diff checks pass.

## Limits

The comparison is deterministic line-level evidence, not semantic equivalence.
Text/Markdown/CSV/JSON/HTML/SRT work without an external engine; PDF and image
snapshots need the already-declared local extraction/OCR tools. Snapshot capture,
live monitoring and scheduling remain separate work.

Windows result access remains deferred to Windows-PC verification.
