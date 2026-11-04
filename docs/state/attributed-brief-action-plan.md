# Attributed brief and action plan

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0075](../plans/0075-attributed-brief-action-plan.md)

**Project tools → Create outputs → Brief + action plan** selects 1–8 frozen
Project sources and a title. It adds a typed request to Chat; it does not send,
prepare, start work or present the staged empty content arrays as a finished
brief.

The provider may draft findings, questions and actions, but the local
`documents.brief` worker is the publication boundary. Every finding and action
must identify one selected source and retain a complete literal quote from that
source. The worker independently locates the quote and computes its line. One
fabricated quote rejects the whole job before either artifact is published.

Questions carry both the unresolved question and why the supplied notes do not
answer it. Actions remain open and attributable. Owner and due date are nullable;
the editable document renders them as **Unassigned** and **Not set** rather than
inventing an assignment or deadline. Non-null dates must be real ISO calendar
dates.

Supported UTF-8 text, Markdown, CSV, JSON, HTML and SRT sources are read directly.
PDFs use configured `pdftotext`; PNG/JPEG/WebP sources use configured Tesseract.
The operation installs nothing and reports missing tools honestly.

One checked request produces:

- `document.docx`, an editable brief with findings, open questions and actions;
- `brief.json`, with source/input-text hashes, exact evidence, line numbers,
  null owner/date values and reconciled counts.

Outputs expose source labels and hashes, not private owner paths or tool paths.

## Verification

- Focused core coverage passes with two frozen note sources. It checks two
  attributed findings, one unresolved question, two actions, source hashes,
  evidence lines and explicit null owner/date values. The generated DOCX is
  reopened through the production inspector and its source labels, question and
  missing-value copy are checked.
- The same test rejects fabricated evidence and the impossible date
  `2027-02-29` before publication.
- Focused desktop coverage passes at 800×560 and proves two supported sources,
  an excluded binary, empty provider-fill arrays, the evidence instruction and
  zero direct Host commands.
- Retained native evidence: [Turkish dark brief form](../screenshots/attributed-brief/attributed-brief-tr-dark.png),
  SHA-256 `48e1803aec425292879195886dd952b2179cf3401ecb812de7d328d693cf69e2`.
- The full workspace passes with 553 tests and 6 explicit fixture/manual
  ignores: `patchfleet-core`'s library has 115 passing tests with 3 ignores and
  the desktop has 252 passing tests. Strict Clippy, formatting, the retained
  screenshot checksum and diff checks also pass.

## Limits

Literal evidence proves traceability, not that a synthesized summary or action
is correct, complete or well prioritized. The owner must review content and
assignments. The editable brief is capped at 500 paragraphs. PDF/image support
depends on owner-installed configured tools.
Windows result access remains deferred to Windows-PC verification.
