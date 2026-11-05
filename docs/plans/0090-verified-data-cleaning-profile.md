# Task0090: Verified data cleaning and profile

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card closes the owner-visible deterministic T1/T2
gap without a live provider/model call, account action, dependency installation
or publication.

## Problem

The local worker can trim CSV cells, remove duplicate rows, calculate selected
numeric columns and package XLSX, but the native Create outputs studio cannot
freeze those choices. Its analysis also lacks source/output hashes, a count of
trimmed cells, per-column missing counts and explicit reopen/match evidence.

## Scope

1. Add one native form for an exact Project CSV, optional exact-row
   deduplication, selected numeric column names and CSV or XLSX output.
2. Reject empty/duplicate headers and invalid numeric-column requests before
   publishing output.
3. Keep existing operations and filenames, while extending `analysis.json` with
   source/output hashes, trimmed-cell count, missing counts and verification.
4. Re-read the emitted CSV or XLSX package and prove cleaned row/column shape.
5. Retain a real source, cleaned CSV, editable XLSX, analyses and native capture.

## Non-goals

- No automatic type inference, fuzzy duplicate matching, imputation, statistical
  significance, forecast, trend narrative or arbitrary formulas.
- No claim that descriptive statistics explain causation or business meaning.
- No live provider/model/account call or dependency installation.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Invalid/duplicate headers and unknown numeric columns fail without publishing
  a partial result; desktop tests prove exact source/policy freezing and no Host
  action.
- A retained CSV produces checked cleaned CSV and editable XLSX outputs whose
  hashes, shapes, transforms, missing values and numeric statistics are pinned.
- The form stays reachable at 800×560 in Turkish and English and both themes.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.

## Outcome

The native exact-CSV form, bounded worker verification, retained CSV/XLSX
evidence and office-reader proof are complete. The full workspace passed 591
tests with 13 explicit ignores and no failures; strict Clippy, formatting,
Python syntax and diff checks also pass. See
[verified data cleaning and profile](../state/verified-data-cleaning-profile.md).
