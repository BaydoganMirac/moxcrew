# Task0077: Source-pinned document translation and rewrite

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances D4 locally without a live provider/model call, account
action, dependency installation or Windows claim.

## Problem

D4 promises translation or style revision without losing meaning or source
version. A generic prompt can produce fluent text, but it cannot prove which
frozen source was used, whether a paragraph disappeared or whether a date, URL,
email address, number or named protected term changed silently.

## Scope

1. Add `documents.rewrite` for one frozen UTF-8 TXT/Markdown or DOCX source.
2. Require one ordered result for every non-empty source line or DOCX body
   paragraph, retaining each literal source segment for comparison.
3. Preserve detected numbers, dates, URLs and email addresses plus up to 30
   owner-selected literal terms.
4. Produce an editable DOCX, source/result Markdown comparison and JSON audit
   bound to the exact input SHA-256 without private owner paths.
5. Add a localized native translation/style form that stages the request in
   Chat without sending, preparing or starting work.

## Non-goals

- No automatic claim that translation meaning, tone or grammar is correct.
- No preservation of arbitrary DOCX formatting, headers, footers, tables,
  comments or content controls.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- Deterministic coverage proves exact ordered segment coverage, input hash,
  clean editable DOCX output, paired comparison and audit without private paths.
- Changed source text, dropped protected facts and no-op rewrites reject before
  publication.
- Both UTF-8 text and DOCX body paragraphs are exercised.
- The native 800×560 form exposes mode, target and protected terms, stages an
  empty provider-fill segment list and issues zero Host commands.
- Retained native evidence is inspected; full workspace, strict Clippy,
  formatting, checksum and diff checks pass.
