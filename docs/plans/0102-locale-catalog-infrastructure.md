# Task 0102: Locale catalog infrastructure

Status: Complete

Updated: 2026-11-06

Authorization: the owner asked to remove scattered language branching and
make the desktop ready for substantially more locales. This card authorizes
repository code and tests only; it does not authorize machine-translating or
shipping eight unreviewed languages.

## Problem

The desktop has one centralized Turkish table, but locale metadata, preference
validation and the selector still encode `tr`/`en` independently. A ten-locale
product would require repeated code edits and make unsupported values easier to
handle inconsistently.

## Scope

1. Define one supported-locale registry with stable code, native label and
   fallback metadata, and use it for preference validation and the selector.
2. Route visible copy through one catalog lookup with English source fallback.
   Keep owner/model/provider text literal and preserve placeholder insertion.
3. Keep current reviewed English and Turkish behavior unchanged. Adding a
   future locale must be a registry/catalog change, not a screen-level branch.
4. Add tests for registry uniqueness, unknown-locale fallback, preference
   round trips and placeholder safety.

## Acceptance

- Production UI code has no locale-specific selector list or `tr`/`en`
  preference allowlist outside the locale module.
- Every registered locale has one stable code and non-empty native label.
- Unknown stored/requested codes fail safely to the documented default or
  return `INVALID_DESKTOP_LANGUAGE` at the mutation boundary.
- Existing English/Turkish snapshots and translation tests remain green.

## Limits

This task builds the infrastructure for ten or more locales; it does not claim
unreviewed translations, locale-aware plural rules, translated owner content
or automatic translation of provider output.

## Completion evidence

- `moxcrew-core::locales` is the single registry consumed by preference
  validation and the native selector. Unknown mutations still fail closed.
- `Language::all()` drives the selector and catalog lookup retains English
  source fallback, reviewed Turkish overrides and post-lookup placeholder
  insertion.
- Registry uniqueness/default tests, preference persistence and all four
  focused desktop i18n tests pass.
