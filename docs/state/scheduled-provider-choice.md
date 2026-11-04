# Scheduled provider choice

Updated: 2026-11-04

Status: Complete; full workspace and strict quality gates pass.

Plan: [Task0061](../plans/0061-scheduled-provider-choice.md).

Recurring recipe schedules previously omitted provider ownership and prepared
every occurrence with the core's historical Codex default. The new optional
durable `providerId` freezes the explicit desktop selection for future
occurrences. Old records without the field remain valid and retain Codex.

The schedule form exposes Codex, Claude Code and Gemini CLI through the shared
provider picker. Creation is available only when current connection diagnostics
show the selected CLI installed, control-compatible and not known to be signed
out. Each schedule card names its saved provider.

The durable test creates a Claude schedule, verifies its first occurrence's
actual workflow assignment, updates the same schedule to Gemini and verifies the
second occurrence uses Gemini. Replay rejects a provider changed only in the
saved fact. Unit coverage proves an old provider-less schedule falls back to
Codex, an older update preserves a newer saved provider and unknown providers
are rejected. Native coverage proves a ready Claude selection enters the owner
payload while an unavailable Gemini selection cannot emit a save command.

The complete workspace/all-target suite passes **524 tests, zero failures and
six ignored opt-in tests**. Strict workspace/all-target/all-feature Clippy with
warnings denied, workspace formatting and `git diff --check` pass. Existing
receipt, restart, overlap, occurrence-budget, timestamp-normalization, skipped
interval and minimum native-layout coverage remains green.

Schedules still run only while Moxcrew is awake and skip missed intervals; this
slice adds no OS wake service. Connection diagnostics prove compatible local
control surfaces, not live account/model access or quota. No provider login,
model turn, Cloud field, Windows claim, signing or publication was added.
