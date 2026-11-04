# Distinctive desktop flows

Updated: 2026-11-04

Status: Implemented and locally verified; owner visual judgment pending.

## Outcome

The Anthropic frontend-design direction now continues beyond the Chat welcome.
Projects, Memory, Decisions, Tasks, Teams, Agents, Providers and Cloud share a
deliberate workbench hierarchy: clear page direction, flat collection ledgers,
one semantic state stripe and bounded surfaces only where the owner is entering
or reviewing consequential information.

Projects uses one aggregate band and a project ledger. Memory sources, local and
shared decisions, queued tasks, saved coding teams, agent packs and providers use
the same record grammar. Selected decisions keep a dedicated inspector. Quick
Start, task creation and account setup use orange guide surfaces because they
are owner-input flows. Decorative all-caps, middle-dot metadata and shortcut
numbering were removed; real step numbering remains.

## Boundaries

No Cloud source, core protocol, event, schema, provider behavior, automatic
execution, installation or publication changed. Existing exact revision,
receipt, provider, Prepare/Start, acceptance and publication authority remains
unchanged. There is no new dependency, gradient, glow or ornamental animation.

## Verification

- `cargo test --locked -p patchfleet-desktop --all-features`: 231 passed. Two
  process-spawning bridge tests were rerun outside the restricted filesystem
  sandbox after macOS denied temporary execution there; both passed.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt --all -- --check`, `cargo check --locked -p patchfleet-desktop
  --all-features` and `git diff --check`: passed.
- Native framebuffer captures exited normally for Projects, Memory, Decisions,
  Tasks, Teams, Agents, Providers and Cloud in Turkish/dark normal windows.
  Projects, Decisions, Tasks and Agents also passed representative English/light
  800×560 review. Evidence is under the sibling
  `output/branding/moxcrew/distinctive-flows/` directory.

The first visual pass exposed an English singular/plural defect in project
counts. It is fixed with a focused regression test and the Projects captures
were regenerated. The evidence does not claim moderated usability, signed
distribution, installed-app replacement or the owner's visual acceptance.
