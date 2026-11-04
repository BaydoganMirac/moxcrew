# Distinctive desktop workbench

Updated: 2026-11-04

Status: Implemented and locally verified; owner visual judgment pending.

## Outcome

The native Rust/egui desktop application now has a deliberate Moxcrew workbench
instead of another generic dark dashboard. The empty Chat state is a high-energy
cobalt launch pad with an orange Mox stage, editorial headline, aligned composer
and direct starting intents. Once work begins, the interface becomes quieter:
owner requests, Mox plans and mission states use one semantic stripe instead of
stacked generic cards.

The shell is denser at 184 points with a 56-point narrow rail. Navigation groups,
brand metadata and task details use sentence case. Project selection,
Chat/Crew/Outputs navigation, the contextual inspector and composer share a
clear reading axis. Turkish/English, light/dark, keyboard behavior, 800×560 and
normal-window behavior retain the existing product contracts.

## Boundaries

No Cloud source, core protocol, event, schema, provider behavior, automatic
execution, installation or publication changed. Prepare and Start remain
separate owner actions. Exact Project, provider, receipt, source and artifact
revision checks remain intact. There is no new production dependency, gradient,
ambient glow or ornamental animation.

## Verification

- `cargo test --locked -p patchfleet-desktop --all-features`: 230 passed.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt --all -- --check`, `cargo check --locked -p patchfleet-desktop
  --all-features` and `git diff --check`: passed.
- Native framebuffer fixtures exited normally with provider probing disabled:
  Turkish/dark normal welcome, English/light 800×560 welcome and Turkish/dark
  populated plan. Accepted evidence is under the sibling
  `output/branding/moxcrew/distinctive-native/` directory.

The first minimum-window review found that the brand hero pushed the starting
intents below the visible area. The final layout reserves an explicit hero row;
the recaptured 800×560 frame keeps the composer footer and all three intents
visible. A transient blank framebuffer retry was rejected rather than counted as
visual evidence.

This establishes bounded technical and native-rendering evidence. It does not
claim moderated usability, signed distribution, installed-app replacement or
the owner's visual acceptance.
