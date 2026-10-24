# Task0053: Apply the UX reference to native screens and flows

Status: Approved by owner; implementation in progress

Updated: 2026-10-25

Authorization: “Tamam knk şimdi bu kurallara göre ekranları ve akışları düzelt,
chip’leri de ekle. Sadece dokümanda kalmasın, implemente et.”

## Outcome and scope

Apply [UX-RULES.md](../../design/moxcrew/UX-RULES.md) to the existing native
application: a readable message composer, removable attachments/selection chips,
distinct suggestion/filter/status roles, contextual action footers, adaptive
Project context, and clear request → plan → Prepare → explicit Start → output
→ revision flows. Preserve all available capabilities and exact existing
provider, Project, receipt, immutable file and side-effect approval contracts.

No new framework, dependency, backend authority, protocol, feature or schema is
needed. Keep current regular/compact metrics; introduce compact chip geometry as
a separate component, with keyboard semantics and non-overlapping hit targets.

## Ownership

- Coordinator/implementation owner: studio.rs shared chips/action helpers,
  chat.rs composer/context, ui.rs command availability, conversation/crew/output
  integration, i18n, smoke fixtures, design reference/status and final validation.
- Bounded builder: capabilities.rs attachment/selection presentation and Project
  tool forms, including tests in that file. No other active owner's files.
- Context builder: crew.rs, output_studio.rs and delivery.rs only, for plan/stage hierarchy,
  contextual output filters/actions and their tests; no composer/shared helpers.
- Independent reviewer: stable source commit and actual captures after integration;
  read-only findings unless fixes are explicitly reassigned.

## Acceptance

1. Files and selected output context are removable, named chips. Overflow opens
   an inspectable bounded list. Removing an item changes only the scoped draft.
   A selection/attachment conflict has visible resolution outside a clipped list.
2. Attach is directly visible at the leading composer edge, primary Send/Prepare
   at the trailing edge. Mode and disabled/pending reasons are understandable.
   Cmd/Ctrl+Enter retains the same validated operation; no implicit Start.
3. Local message editing/removal/navigation remains usable during Host work;
   conflicting commands, automatic reads and pending receipt transitions cannot
   be staged or lost while the bridge is busy.
4. The Project context adapts to available conversation/detail width and can be
   resized when both fit. Narrow Back to chat restores the same draft/selection.
5. Plan and prepared work show the correct stage action once per task context.
   Outputs provide contextual filters, preview and deliberate revision. Project
   tool forms put values before commit actions and keep required actions reachable.
6. Actual rendered tests cover TR/EN, light/dark, 800×560 and normal windows,
   long names, 12 files, conflicts, missing connection, busy and panel transitions.
   Test chip removal/focus/keyboard, scoped payloads and preserved draft behavior.
7. Run desktop tests, formatting and strict Clippy; inspect real native captures
   for the changed states. Record limitations honestly: captures are not moderated
   usability or native screen-reader evidence. Build a local development package
   if the available native packaging workflow succeeds; no push or installation.

## Handoff

Update [implementation state](../state/ux-rules-implementation.md), the UI kit
and indexes. Provide local commits, validation results, actual captures and
remaining platform/user-study limits. Critical clipping, hidden conflict
resolution, draft loss and command/authority regressions block completion.
