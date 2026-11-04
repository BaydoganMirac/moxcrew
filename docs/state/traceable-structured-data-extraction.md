# Traceable structured data extraction

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0074](../plans/0074-traceable-structured-data-extraction.md)

**Project tools → Create outputs → Extract table** selects 1–8 frozen Project
sources, a title and 1–12 `key: label` columns. It adds a typed request to Chat;
it does not send, prepare, start work or claim that the staged empty rows are a
finished extraction.

The provider may propose rows, but the local `data.extract` worker is the
publication boundary. Every cell must be exactly one of:

- `confirmed`: non-empty value and literal evidence from the same frozen source;
- `uncertain`: the same literal-evidence requirement, retained as uncertain; or
- `missing`: null value and null evidence, never a guessed placeholder.

For confirmed and uncertain cells the value itself must occur inside the quoted
evidence. The worker independently locates that complete quote in the extracted
source text and computes its line. One invalid cell rejects the whole job before
any artifact is published.

Supported UTF-8 text, Markdown, CSV, JSON, HTML and SRT files are read directly.
PDFs use the configured `pdftotext`; PNG/JPEG/WebP sources use configured
Tesseract. The operation installs nothing and reports missing tools honestly.

One checked row set produces:

- `extracted-table.csv`, with value, status and evidence-line columns;
- `extraction.json`, with source/input-text hashes, exact evidence, line numbers,
  schema, rows and confirmed/uncertain/missing totals; and
- `report.md`, with escaped labels and per-source status coverage.

Outputs expose source labels and hashes, not private owner paths or tool paths.

## Verification

- Focused core coverage passes with three frozen sources: direct UTF-8 text, a
  protocol PDF extractor and a protocol OCR extractor. Nine cells reconcile as
  5 confirmed, 2 uncertain and 2 missing; exact line evidence and output files
  are checked. Fabricated evidence and a value hidden inside a missing cell are
  rejected.
- Focused desktop coverage passes at 800×560 and proves two supported sources,
  an excluded binary, exact schema keys, an intentionally empty provider-fill
  row list, the evidence instruction and zero direct Host commands.
- Retained native evidence: [Turkish dark extraction form](../screenshots/data-extraction/structured-extraction-tr-dark.png),
  SHA-256 `0946ea0a20d124922d493ec2d1298ebcfa29deaed153f7d726c76912b2683a32`.
- The full workspace passes with 551 tests and 6 explicit fixture/manual
  ignores: `patchfleet-core`'s library has 114 passing tests with 3 ignores and
  the desktop has 251 passing tests. Strict Clippy, formatting, the retained
  screenshot checksum and diff checks also pass.

## Limits

This proves traceability of literal extracted text, not source truth, OCR
accuracy, semantic normalization, deduplication or model quality. A non-missing
value must be a literal substring of its complete evidence quote. PDF/image
support depends on owner-installed configured tools. Windows result access
remains deferred to Windows-PC verification.
