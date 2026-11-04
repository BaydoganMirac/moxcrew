# Task0061: Scheduled provider choice

Status: Complete; full workspace and strict quality gates pass

Updated: 2026-11-04

Authorization: continue closing non-Windows product gaps while Windows remains
deferred for native hardware testing.

## Outcome

Creating a recurring recipe schedule requires an explicit ready provider and
freezes that provider into the schedule. Every occurrence uses the saved Codex,
Claude Code or Gemini CLI selection instead of silently defaulting to Codex.

## Contract

- Add an optional, validated `providerId` to `project_schedule_save` and the
  durable schedule record.
- New desktop requests always send the visible selection. Existing records and
  older requests without the field retain their historical Codex default.
- Schedule updates preserve the saved provider when an older client omits it.
- The schedule card shows the provider; create remains disabled until the
  selected CLI has proven installation, control compatibility and acceptable
  authentication readiness.
- Occurrence identity, budgets, awake-only execution, missed-interval skipping,
  receipts, replay and overlap rules remain unchanged.

## Verification

- Prove the owner payload, durable schedule and prepared occurrence agree on
  the selected provider.
- Prove tampering with the saved provider fails replay authorization.
- Prove legacy schedules without `providerId` still replay and run as Codex.
- Cover the native selection, readiness gate, localization and minimum layout.
- Run focused core/desktop tests, full workspace tests, strict Clippy, formatting
  and diff checks.

## Boundaries

No unattended host wake, OS background service, provider sign-in, model call,
new provider permission, Windows claim, Cloud field, signing or publication.

## Completion

The schedule form now freezes a ready Codex, Claude Code or Gemini CLI selection,
and each prepared occurrence carries that provider into its agent assignments.
Legacy records without the optional field replay with Codex; an older client
updating a new record preserves its saved provider. Tampered/unknown providers
fail closed.

Focused core and native tests pass. The complete workspace/all-target suite
passes **524 tests with zero failures and six opt-in tests ignored**. Strict
workspace/all-target/all-feature Clippy (`-D warnings`), workspace formatting
and `git diff --check` pass. No live provider account/model call was made.
