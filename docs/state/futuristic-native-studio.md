# Futuristic native studio

Status: Complete; independently reviewed and locally verified

Updated: 2026-10-19

Owner-approved [Task0042](../plans/0042-futuristic-native-studio.md) revises the
desktop's visual composition after the initial Project conversation.

## Implementation

- Ink/cobalt semantic theme revision 1.1, cool light surfaces, locally rendered
  ambient light, compact navigation and native category icons.
- The existing Mox identity sits in an orbital welcome stage. Its active motion
  still requires real running work, visibility and reduced motion disabled.
- Centered welcome and conversation, capability cards, quieter header and
  floating composer. File and connection surfaces follow the same hierarchy.
- Existing Project, provider, prepare/start, follow-up, results and receipt
  contracts remain; no persistence, event or Cloud protocol changes.

## Evidence

Forty-four semantic contrast pairs pass: normal text at least 4.5:1 and defined
control/focus boundaries at least 3:1. Native previewing caught and corrected
minimum-sidebar wrapping and category-grid overflow before final verification.
All 128 desktop checks pass, including minimum-window/card containment, actual
keyboard selection, preserved drafts/replies and shared-control focus without
implicit activation. Formatting and strict desktop all-target/all-feature
Clippy pass. The final feature-off release build passes.

Independent review accepted `78ddc1b`, after fixing the only P2: explicit button
fills hid the native keyboard-focus frame. Shared controls, primary submit and
command buttons now draw a visible focus outline; the new test inspects its
rendered shape in both themes and verifies explicit Enter activation.

Thirty-five actual native GL frames at `4b630cd` cover TR/EN, dark/light,
1280x880 and 800x560, empty Project/welcome, history, files, connections and
reduced motion. The subsequent focus-only correction is covered by the rendered
focus regression. Captures use disposable Host data and real fixture files;
they are not evidence of a generated presentation or live provider execution.

- [Welcome, Turkish dark](../screenshots/futuristic-studio/chat-compose-tr-dark.png)
- [Welcome, English light](../screenshots/futuristic-studio/chat-compose-en-light.png)
- [Minimum welcome](../screenshots/futuristic-studio/chat-compose-tr-dark-min.png)
- [Project history](../screenshots/futuristic-studio/chat-history-tr-dark.png)
- [Actual file panel](../screenshots/futuristic-studio/chat-results-en-light.png)
- [Minimum connections](../screenshots/futuristic-studio/chat-connections-tr-dark-min.png)

## Development package

The macOS ARM64 build is ad-hoc signed, without public notarization. The final
feature-off App at `78ddc1b` passed strict signature, bundle identity/version,
license and absent smoke-control checks. An isolated NSWorkspace launch with
temporary data and failing provider stubs found its own native window, closed
normally and released the writer lock. No installed application or owner data
was replaced. Visual evidence above uses GL readback, not an OS production
screen capture.

`dist/native/futuristic-studio/Moxcrew_0.12.0_studio_aarch64.dmg` is 7,563,456 bytes;
SHA-256 `26596702f93d9b0d82719f7d37520f431a213a84296689dcddd73ded8506bc9c`.
The packaged executable is 14,512,448 bytes; SHA-256
`8525b8a09dcdf9358e5c1089ea1711e43fe23ff2d083d44faaf934919e209064`.
DMG integrity verification passes. The six-second empty launch sampled
112,736 KiB RSS and 0.14 seconds cumulative CPU; this is a startup observation,
not a sustained performance guarantee.

## Limits

Codex remains the only execution provider. The forty-ability release and native
Windows/Linux runtime proof remain separate work. No live provider turn, Cloud
deployment, publication or user-data migration is part of this visual revision.
Manual screen-reader use and OS keyboard navigation were not exercised; native
widget accessibility and keyboard activation were checked at the egui boundary.
