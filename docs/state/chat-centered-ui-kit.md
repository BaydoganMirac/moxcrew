# Shared native UI kit and chat-centered studio

Updated: 2026-10-19

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0048](../plans/0048-chat-centered-project-studio.md).
Decision: [ADR0047](../decisions/0047-chat-centered-project-context.md).
Component contract: [Moxcrew UI kit](../../design/moxcrew/UI-KIT.md).

The owner selected chat-centered interaction and explicitly requested a kit to
correct inconsistent buttons, padding and spacing. Shared native helpers now
use semantic button variants, 44/36 px controls, a 4 px spacing scale, common
input insets and card/page metrics. Hover/focus/disabled states belong to the
same implementation. Private inputs and command/validation behavior are retained.
Buttons are measured directly in their parent row so long labels move together
instead of breaking into vertical words. Explicit semantic label colors survive
the global body-text override; a painted-text regression checks both themes.

Crew/Outputs reuse their actual controls in one right context at normal/wide
windows. The conversation and composer remain present. Narrow windows show the
context at readable width with Back to chat. Project changes close the context;
resizing does not change its state or command authority. Context inputs keep
global UI identities across narrow/wide layouts. Existing Teams and
file-follow-up routes remain available. No core/provider/Cloud change.

The composer groups supporting controls on a second row when its actual inner
width needs it. Its reserved height uses the same frame's border/padding insets,
including the responsive boundary around 1360–1400 px. Crew preparation and
delivery footers place real actions first and let supporting text wrap within
the remaining width. Saved-plan goal adoption remains explicit and visible
before role cards; opening a saved plan does not replace an unsent goal.

## Verification

- Complete all-feature desktop suite: **169 passed, 0 failed, 0 ignored**.
  The subsequent inset-only correction passed the affected resize regression
  in TR/EN and both themes, including 1280→1360→1392→1400→800→1280 widths.
- Shared dimensions are checked against the brand JSON, and control heights,
  disabled state, actual semantic foreground colors, keyboard focus and activation
  are exercised. Existing file hashes, frozen approval, late receipts, private
  credential clearing and explicit Prepare/Start checks remain green.
- Workspace/all-target/all-feature strict Clippy and workspace formatting pass.
- Final implementation: `85c0805`, following `3bb04d1`, `686bacd` and `263b9ef`.
- **42 native captures**: the shared component gallery in light/dark; Chat,
  Crew, Outputs and delivery selection in TR/EN, both themes and minimum/normal
  windows; Decisions, Projects, Providers and frozen app approval in TR and both
  themes at minimum size. Native source and output review passed with no
  remaining blocker. Gallery layout fits the actual OS-constrained window.
  Local evidence is in the sibling output directory
  `branding/moxcrew/ui-kit-qa/native/qa.json`.
  Fixture binary SHA-256:
  `d5355e170df37d867519b91eb9d258f47f7f9c80b89954c45bdc813997e6402b`.

## Development package

`dist/native/ui-kit/Moxcrew_0.12.0_ui-kit_aarch64.dmg`

Version remains 0.12.0. The separate macOS ARM64 package is ad-hoc signed,
not publicly notarized. License resources, strict signature verification,
exclusion of native fixture/gallery code, normal launch/close, writer-lock
release and DMG integrity verification pass. Evidence:
`dist/native/ui-kit/qa.json`.

- Executable: 16,538,736 bytes; SHA-256
  `816872aacc35615f964b9b9cbaabb908bfee3d7bd7b7c77de36f6979774911c0`.
- DMG: 8,522,999 bytes; SHA-256
  `6015504ec899abc647a3cc307626f4cd6f056b3d0f05f82932d70e340c493442`.

The native gallery is a feature-gated reference, not a production navigation
item. Public signing, push and deployment are not part of this change.
Live inference, real external-account writes and native
Windows/Linux execution were not tested in this UI change.
