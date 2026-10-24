# Task0054: Desktop composition and generated artifact cleanup

Status: Owner requested; implementation in progress

Updated: 2026-10-25

Authorization: owner asks to remove unnecessary packages because disk is full,
and corrects the premium design direction: the problem is layout, not animation.

## Scope

Remove reproducible Cargo build artifacts and obsolete native package copies;
retain source, toolchains, dependencies, Project data and brand masters. Keep one
existing App as a fallback. Build only the debug/test executable needed for this
UI change, with incremental compilation and debug symbols disabled. No new
release bundle or DMG.

Recompose the native shell, welcome, conversation/composer and Project context:
compact desktop chrome, a readable central conversation, supporting inspectors,
fewer nested containers and a clear difference between navigation and actions.
Use 36-point regular controls and 32-point compact controls/chips; do not shrink
the body font or keyboard targets. Keep all capabilities and the exact existing
Project/provider/file/receipt/Prepare/Start/side-effect contracts.

## Ownership and acceptance

- Coordinator: cleanup, studio.rs, ui.rs, chat.rs, translations, tokens, docs,
  integration and final capture selection.
- Bounded context builder: crew.rs, output_studio.rs and delivery.rs only, to
  reduce nested/full-width forms, duplicated labels and competing action rows.
- Read-only reviewer after stable source: check actual native images and command
  invariants. No concurrent edits or duplicate builds.

Record actual disk space reclaimed, test results and a small useful set of new
native captures. Review normal and minimum windows with real fixture content.
Geometry tests alone cannot establish premium aesthetics or usability. Preserve
named keyboard controls, visible error/conflict resolution and scoped drafts.
No animation work, provider changes, new dependency, framework migration, Cloud
deployment, installation or package publishing is part of this task.
