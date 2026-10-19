# Task0049: Apply the UI kit across native screens

Status: Owner-approved, implementing

Updated: 2026-10-19

Authorization: the owner explicitly requested updating the application screens
to the shared Moxcrew UI kit after reviewing its native component reference.

## Observable result

- Apply the kit to the nine native destinations: Chat, Projects, Teams, Tasks,
  Project Memory, Decisions, Agents, Providers and the optional account/Cloud UI.
  Retain the accepted chat-centered Crew/Outputs context.
- Replace remaining bespoke card insets/radii, form fields, direct action
  buttons and off-scale layout gaps with the shared components and measurements.
  Page and section headings follow one hierarchy. Search/filter rows and form
  actions remain usable at800x560 and normal widths in TR/EN and both themes.
- Provider setup, project/folder forms, task/team composers, source/decision
  forms, crew/catalog dialogs, output previews/revisions and frozen external
  approvals use the same kit. A screen-specific necessary compact layout is
  explicit; painter coordinates, status badges and icon geometry are not
  indiscriminately replaced with spacing tokens.
- Preserve widget identities, unsent drafts, focus, Project/file/hash/revision
  scopes, private token clearing, exact receipt handling and explicit approval.
  No core/provider/Cloud behavior, new feature, dependency or schema change.

## Ownership and acceptance

Coordinator owns ui.rs, studio.rs, main.rs, i18n.rs, shared docs, fixtures and
native/package integration. Builder owns create.rs, chat.rs, connect.rs, crew.rs,
output_studio.rs and delivery.rs for Project surfaces only. Reviewer is read-only
after stable source and actual captures. Do not edit another owner's files.

Inventory each destination/dialog and record what changed. Reuse existing
controls instead of adding another theme or component framework. Run the complete
desktop suite, focused regressions for any discovered clipping, strict workspace
Clippy/formatting, actual native captures of all destinations and key dialogs,
and a separate smoke-free macOS development package. Record remaining platform
and live-provider limits. No inference, real account writes, installation, push,
deployment or public release.
