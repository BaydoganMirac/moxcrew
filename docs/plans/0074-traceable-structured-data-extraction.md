# Task0074: Traceable structured data extraction

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances T5 locally without a live provider/model call, account
action, dependency installation or Windows claim.

## Problem

T5 requires supported sources to become a usable table without hiding uncertain
or missing fields. The existing generic text extraction can expose source text,
but it does not bind each structured value to literal evidence or reject a
fabricated row before publication.

## Scope

1. Add `data.extract` for 1–8 frozen text, CSV, JSON, HTML, PDF or image sources
   and 1–12 explicit output columns.
2. Require each non-missing cell to declare confirmed or uncertain status, an
   exact source quote and a value literally contained in that quote. Missing
   cells must stay explicit with null value and evidence.
3. Produce a traceable CSV, evidence JSON and escaped Markdown report with
   source hashes, evidence line numbers and status counts but no owner paths.
4. Use configured `pdftotext` and Tesseract only for PDF/image text; unsupported
   or unavailable formats fail before publication.
5. Add a localized native form that stages frozen sources and a checked schema
   in Chat without sending, preparing or starting work.

## Non-goals

- No inferred truth, fuzzy citation matching, normalization, entity resolution,
  arbitrary regex, silent field filling or OCR-quality claim.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- Deterministic coverage proves direct text, PDF-adapter and OCR-adapter input,
  exact evidence, source hashes/line numbers, confirmed/uncertain/missing counts,
  escaped reports and rejection of fabricated evidence or hidden missing data.
- The native 800×560 form freezes only supported selected resources and exact
  schema keys, issues zero Host commands and visibly explains the evidence rule.
- Retained native evidence is inspected; full workspace, strict Clippy,
  formatting and diff checks pass.
