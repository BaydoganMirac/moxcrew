# Task0045: Native task hierarchy

Status: Complete; independently reviewed and locally verified

Updated: 2026-10-19

Request: “Knk ekran görüntülerine bakıyorum da tasarım hiyerarşi felan rezalet”.
This corrects the usability of Task0044's delivered native features. It adds no
runtime capability or persistence schema.

## Observable result

- Keep Chat as the default. Project-level Conversation, Crew and Outputs views
  have one quiet, consistent navigation row and preserve drafts when switching.
- Crew and Output Studio become native content views inside the existing shell,
  rather than large layered modal control panels. Dialogs remain for bounded
  edits and explicit account/action approval when appropriate.
- Establish clear hierarchy: a short page title, current work/result, one primary
  next action, then secondary details. Completed planning must not duplicate a
  request form, status panel and plan card at equal visual weight.
- Show the agent roster as readable roles/steps, outputs as a file/preview area,
  and resources/settings as secondary views. Avoid walls of cards, raw codes,
  repeated borders and equally prominent buttons. Setup is progressively shown.
- Preserve all twelve features, exact Project/command/file scoping, frozen action
  previews, credential clearing, keyboard/focus behavior and reduced motion.
- TR/EN and both themes must work at 800x560 and a normal desktop window.

## Ownership and checks

Coordinator owns chat.rs, shared studio.rs/ui.rs visual helpers, this card,
integration, native screenshot review and packaging. UI builder owns crew.rs,
output_studio.rs, create.rs, i18n.rs and necessary smoke fixtures/state. Coordinate
new view-selection methods before changing callers. The independent reviewer
assesses actual screenshots after the stable implementation.

Use existing native components and dependencies. Test actual view switching,
primary actions, preservation of draft/scope/token behavior, narrow layout and
explicit approval. Run the affected desktop suite, formatting and strict Clippy;
core runtime remains unchanged. Inspect real native captures before declaring the
visual result complete. Build a separate feature-off macOS development package.

## Completion

The native implementation, three independent review corrections, 158 passing
all-feature desktop tests, strict workspace Clippy/formatting, 52 final captures
and a verified feature-off macOS development package are recorded in the
[hierarchy state](../state/native-task-hierarchy.md). Core schemas and Cloud
contracts remain unchanged. Public release and other-platform verification are
outside this UI correction.
