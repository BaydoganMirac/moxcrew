# Task0050: Conversation first and visible provider selection

Status: Owner-approved, investigating and implementing

Updated: 2026-10-19

Authorization: the owner reports excessive Chat setup and asks to accomplish
work through conversation, and asks why Codex cannot be selected. This authorizes
simplifying the existing flow and diagnosing/fixing actual provider selection.

## Observable result

- The first surface is a conversation composer with one Send action and a visible
  local provider selector (Codex, Claude Code, Gemini CLI). Skill, work-mode,
  roster and detailed planning configuration are optional secondary controls.
- A user can write the first request without creating/selecting a Project or
  choosing a recipe. Reuse existing Project/plan commands, with exact receipt
  handling before any dependent action. Existing Projects and drafts survive.
- Real provider planning questions, plan summaries and revisions appear as chat
  turns. Answer questions through the same composer instead of multiple form
  fields. A plan is inspectable in Crew; execution still needs an explicit Start
  action and exact Project/plan/mission binding. Sending a general message never
  implicitly starts an executing crew or approves an external account write.
- Codex is selectable directly in Chat. Show actionable installation/authentication/
  capability diagnosis independently from selection. Trace actual discovery and
  cached-read conditions; fix evidenced causes without weakening version, auth,
  tool or execution capability boundaries. Do not claim account/model readiness
  from an installed executable.
- Reuse native UI-kit dimensions, saved drafts, scope/file/hash/approval safeguards,
  private clearing and local writer. No new runtime dependency, cloud change,
  credential change, real inference, installation, push or deployment.

## Ownership and acceptance

Coordinator owns product/ADR/contracts, native UI integration and shared docs.
A provider worker may inspect discovery/connection readiness, then receive exact
file ownership for an evidenced fix. A separate reviewer checks stable source
and actual native screens. Keep file ownership disjoint.

Tests must cover first request/no Project, provider choice and readiness,
question/reply/revision continuity, exact late/failed receipts, draft preservation,
no implicit crew start, and narrow TR/EN light/dark control reachability. Run the
complete affected desktop suite and provider tests for changed discovery/readiness,
strict formatting/Clippy, native captures of actual simplified conversation states,
and a smoke-free macOS development package. Record platform/live-provider limits.
