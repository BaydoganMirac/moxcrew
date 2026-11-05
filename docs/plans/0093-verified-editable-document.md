# Task0093: Verified editable document composer

Status: Complete bounded path; semantic and Microsoft Word review pending

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card implements the existing D1 document ability
without a live provider/model call, dependency installation, account action,
publication or cleanup.

## Problem

The D1 catalog entry promises editable reports, proposals and CV-style files,
but the desktop app has no owner-visible create-document form. The bundled
`documents.create` worker produces a minimally valid sequence of unstyled
paragraphs without canonical content, hashes, a verification manifest or
retained render evidence.

## Scope

1. Extend the existing `documents.create` operation with a backward-compatible
   structured document request: language, title, optional subtitle and one to
   twenty owner-supplied sections.
2. Produce a polished, editable DOCX with native Word paragraph styles while
   preserving the exact supplied text and deterministic package bytes.
3. Emit canonical `content.json`, `document-manifest.json` and `summary.md`,
   reopen every output and bind the DOCX/text structure to exact hashes.
4. Add a native Create outputs form that stages the exact request in Chat and
   never starts production, invents prose or publishes a file by itself.
5. Retain a real LibreOffice render and native Turkish dark UI capture, with
   explicit human-review and Microsoft Word compatibility limits.

## Non-goals

- No model-written copy, research, fact completion, CV scoring or semantic
  quality claim.
- No arbitrary Word XML, macros, remote templates, external references or
  embedded active content.
- No live provider, publication, cloud upload, account action or cleanup.
- No claim about Microsoft Word, assistive-technology or Windows behavior.

## Acceptance

- Unexpected fields, empty sections, duplicate/invalid structure and bounded
  input violations fail without a result; legacy paragraph requests still work.
- Tests prove exact reopened text/style structure, deterministic DOCX bytes,
  canonical content and recomputed manifest hashes.
- Desktop tests prove exact request staging, no direct Host action, and form
  reachability at 800×560 in Turkish and English and both themes.
- Retained evidence includes the real DOCX package, canonical/manifest files,
  a LibreOffice-rendered page and native form capture.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.

## Outcome

The backward-compatible typed worker, deterministic styled DOCX package,
canonical content and hash manifest, Turkish/English native form, retained real
LibreOffice render and native Turkish dark capture are complete. The full
workspace passed 599 tests with 16 explicit ignores and no failures; strict
Clippy, formatting, Python syntax and diff checks also pass. Semantic content
quality, Microsoft Word/assistive-technology review, publication and Windows
behavior remain explicit future gates. See
[verified editable document](../state/verified-editable-document.md).
