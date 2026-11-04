# Task0078: Dated source change tracking

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances R5 locally without a live page request, provider/model
call, account action, dependency installation or Windows claim.

## Problem

R5 exists in the ability catalog, but no typed production operation compares two
exact Project sources. A generic prompt can silently use a live page, lose the
capture dates, treat whitespace as a product change or report success without a
reproducible source version.

## Scope

1. Add `research.changes` for two distinct frozen sources with strictly ordered
   owner-supplied ISO dates.
2. Bind both source and extracted-text SHA-256 values into the result.
3. Compare normalized non-empty lines and distinguish added, removed, modified
   and unchanged content with original line numbers.
4. Produce human-readable Markdown and machine-readable JSON, including an
   explicit no-meaningful-change result for identical or formatting-only input.
5. Add a localized native form that stages exactly two supported resources and
   dates in Chat without sending, preparing or starting work.

## Non-goals

- No live URL retrieval, crawling, scheduling or automatic snapshot capture.
- No semantic claim that two differently worded sentences mean the same thing.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- A deterministic changed pair yields dated, source-hashed additions,
  modifications, removals and unchanged counts with line-bound review entries.
- A formatting-only pair succeeds with `hasChanges: false` and an explicit
  no-meaningful-change explanation.
- Reversed/equal dates, duplicate sources, unsupported formats and bounded-input
  violations reject before publication.
- The 800×560 native form stays visible, requires distinct supported snapshots
  and increasing dates, and emits zero direct Host commands.
- Retained native evidence is inspected; focused/full tests, strict Clippy,
  formatting, checksum and diff checks pass.
