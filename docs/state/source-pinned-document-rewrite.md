# Source-pinned document translation and rewrite

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0077](../plans/0077-source-pinned-document-rewrite.md)

**Project tools → Create outputs → Translate or rewrite** selects one frozen
UTF-8 TXT/Markdown or DOCX resource, a translation/style target and optional
comma-separated protected terms. The form adds a typed request to Chat; it does
not send, prepare, start work or present its empty segment list as a finished
translation.

The provider reads the exact frozen source and proposes one result for every
non-empty text line or DOCX body paragraph. The local `documents.rewrite` worker
is the publication boundary. It rejects missing, reordered or changed source
strings, more than 200 segments, more than 100,000 source characters and a result
that changes any detected number/date, URL, email address or requested literal
term. At least one segment must actually change.

One checked request produces:

- `document.docx`, a clean editable result containing the transformed segments;
- `rewrite-comparison.md`, with every literal source/result pair and source hash;
- `rewrite-audit.json`, with mode, target, input SHA-256, format, segment count,
  automatically protected facts, owner-selected terms and exact pairs.

The result exposes no private owner path or tool path. The source/result pairing
supports semantic review; it is not an automatic meaning-equivalence score.

## Verification

- Focused core coverage translates a three-line UTF-8 source, proves the exact
  SHA-256, retained date/URL/budget/name, editable DOCX text, paired Markdown and
  audit. It rejects changed source text and a dropped URL.
- The same coverage rewrites a generated real DOCX and proves exact body
  paragraph coverage plus protected name/date retention.
- Focused desktop coverage proves one supported selected source, one excluded
  PDF, mode/target/protected terms, empty provider-fill segments, the explicit
  owner-review instruction and zero direct Host commands.
- Retained native evidence: [Turkish dark rewrite form](../screenshots/document-rewrite/document-rewrite-tr-dark.png),
  SHA-256 `a93450fe4ab0dd1d808331ff9361a1b7a3162cc4351d505b8d565099a0f196da`.
- The full workspace passes with 557 tests and 6 explicit fixture/manual ignores:
  `patchfleet-core`'s library has 117 passing tests with 3 ignores and the
  desktop has 254 passing tests. Strict Clippy, formatting, the retained
  screenshot checksum and diff checks also pass.

## Limits

TXT/Markdown units are non-empty lines; DOCX units are non-empty body paragraphs.
The clean result does not preserve arbitrary original Word styling or non-body
content. Automatic protection covers stable literal facts, not names or semantic
relationships; owners should list important names as protected terms and must
review the comparison for meaning, tone and grammar.

Windows result access remains deferred to Windows-PC verification.
