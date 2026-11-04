# Explicit DOCX template filling

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0076](../plans/0076-explicit-docx-template-filling.md)

**Project tools → Create outputs → Fill Word template** selects one frozen
DOCX and accepts one `key | value` per line. A blank value is staged explicitly
as missing. The form adds a typed request to Chat; it does not send, prepare,
start work or present the staged values as a completed document.

The local `documents.template` worker is the publication boundary. It reads the
template as a bounded Office archive, rejects external references and recognizes
`{{field_key}}` in document, header and footer paragraphs. A placeholder may be
split across ordinary Word text runs. The request must contain the exact unique
placeholder set: no field can be omitted or added.

Provided values are bounded plain single-line text. A missing value must carry
JSON null and renders as `[MISSING: field_key]`; it cannot become invisible.
Non-placeholder paragraph text is retained. One checked request produces:

- `filled-template.docx`, the editable filled document;
- `template-fill.json`, with the input hash, field statuses and reconciled
  provided/missing counts.

The result and audit expose the template hash and field values but no private
owner path or tool path.

## Verification

- Focused core coverage creates a DOCX, fills two values, keeps one missing value
  visible, reopens the result through the production inspector and checks the
  audit. It also rejects an omitted field and a non-null missing value.
- A minimal valid DOCX fixture splits `{{client_name}}` between two Word runs;
  the filled result reopens with the expected text.
- Focused desktop coverage proves one selected DOCX, one excluded PDF, explicit
  provided/missing fields, the exact-placeholder instruction and zero direct
  Host commands.
- Retained native evidence: [Turkish dark template form](../screenshots/docx-template/docx-template-tr-dark.png),
  SHA-256 `abcc39b78d5688453fbdf4bbc662cb4c8b2bec2ac0d56fc451b9bb7fbfd7cd29`.
- The full workspace passes with 555 tests and 6 explicit fixture/manual ignores:
  `patchfleet-core`'s library has 116 passing tests with 3 ignores and the
  desktop has 253 passing tests. Strict Clippy, formatting, the retained
  screenshot checksum and diff checks also pass.

## Limits

Placeholders must use lower snake-case keys, occur within one paragraph and be
unique. Body, header and footer text are supported; content controls, comments,
footnotes, mail-merge fields and placeholders spanning paragraphs are not. The
worker preserves template text structure but does not claim Word layout, factual
accuracy or completeness; the owner must inspect the result.

Windows result access remains deferred to Windows-PC verification.
