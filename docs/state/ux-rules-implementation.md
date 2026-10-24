# UX rules implementation

Updated: 2026-10-25

Status: Complete for the approved native UI scope; local development package verified

Plan: [Task0053](../plans/0053-apply-ux-rules-and-chips.md).
Reference: [UX rules](../../design/moxcrew/UX-RULES.md).
Implemented components: [UI kit](../../design/moxcrew/UI-KIT.md).

Follow-up: the owner rejected the visual composition after this bounded technical
remediation. [Task0054](../plans/0054-desktop-composition-and-build-cleanup.md)
recomposes the desktop screens and cleans reproducible artifacts. The DMG listed
below was verified at this historical checkpoint and subsequently removed at the
owner's cleanup request; it is not a current download. One existing App was kept
as a fallback. These test results do not imply owner visual acceptance.

## Delivered behavior

- Shared 32-point removable input, selected filter and optional suggestion chips;
  named keyboard targets, stable attachment identity and bounded `+N` overflow.
  Filter selection uses a vector check, avoiding unsupported font glyphs.
- Direct Attach and trailing Send/Prepare; visible selection/attachment conflict,
  bounded 64-point editor and the same validated operation on Cmd/Ctrl+Enter.
  Revision context is named and replaces the inactive follow-up chip without
  deleting that saved draft. The editor prompts for the requested change.
- Current-content composer reservation uses component metrics and measured
  wrapped feedback, rather than egui Panel's previous natural height. A regression
  showed that cached height moved More between press/release after file-panel
  closure. The existing full follow-up/skill-switch journey now passes.
- Host busy state gates commands, automatic reads and receipt continuations,
  while preserving typing, local chip removal and navigation. Exact pending
  draft and Project/provider/resource/receipt checks remain authoritative.
- Resizable Project context reserves readable conversation width; narrow layouts
  retain the same draft. Crew stage footers, semantic output filters, contextual
  Preview/Revise and secondary export/package actions are integrated.
- Package selection uses one compact toolbar and a bounded review popup. Actual
  file rows take priority over help text. Changed/missing versions stay visible
  in selection review and removable from the keyboard; limits remain under More
  and at the export boundary. The first entire file row fits the minimum window.
- Brand values precede a stable Save footer with dirty/invalid/no-change states.
  Selected logo references outside a loaded resource page are retained.
- TR/EN labels, the shared UI-kit scene and semantic chip token are updated.

## Source and independent review

Local commits:

- `c3f57f0`: attachment/selection chips and Project tool/Brand forms.
- `fd52036`: Crew stages and contextual output/delivery actions.
- `9a3d6ba`: shared components, command availability, composer and integration.
- `544a33d`: minimum-window package file visibility and selection review.
- `b578520`: vector selection check and compact, explicit revision context.

Final runtime source is `b578520f298e6d13dd2ce6017669f5d04547c8de`.
An independent read-only reviewer compared the connected implementation with
`983e0d3`, reviewed the final native contacts and full minimum attachment,
output and delivery images, and found no actionable blocker. The reviewer did
not duplicate the recorded test/build runs.

## Validation and artifacts

Evidence is local under `../output/branding/moxcrew/ux-rules-qa/` relative to the
repository root. It contains fixture labels and disposable local data; no live
account or model was used.

- `desktop-tests-final.log`: **227 passed, 0 failed**, from
  `cargo test --locked -p patchfleet-desktop --all-features`.
- `clippy-final.log`: strict workspace/all-target/all-feature Clippy passed with
  `-D warnings`. Formatting and `git diff --check` pass.
- Tests cover busy typing/removal/navigation, no conflicting dispatch, unchanged
  scoped drafts, keyboard chip/filter/selection actions, visible conflict
  resolution and exact output/package versions. Long-message and panel-transition
  regressions exercise 100/125/150% desktop scale. Minimum package tests assert
  that the whole first row and checkbox are visible, then select by keyboard;
  stale-version tests inspect the original path/hash and remove only that version.
- `native-final/qa.json`: **40 final native framebuffer captures**, all exit 0:
  eight scenes across TR/EN and light/dark at 800×560 logical points (32), plus
  eight TR/dark normal-window scenes at 1280×880 (8). Both contact sheets and
  full changed-state images were visually reviewed. Metadata records the clean
  final runtime source, debug binary hash and each image hash. Earlier `native/`
  captures retain their original `9a3d6ba` provenance; they are not relabeled.
- The first capture review caught an unsupported check glyph and a package list
  buried below controls. Both were corrected before the final matrix. Revision
  context was also compacted without changing its validated payload.
- `release-build-final.log`, `package-final.log`, `package-verify-final.log` and
  `package-qa.json`: release build and local macOS arm64 App/DMG succeed.
  `codesign --verify --deep --strict`, required license files, version 0.12.0,
  absence of fixture markers and `hdiutil verify` pass. The DMG is
  `dist/native/ux-rules/Moxcrew_0.12.0_ux-rules_aarch64.dmg` (9,069,952 bytes),
  SHA-256 `370bf914ced9463ccfb6dc706572b147f6d5107bfacf884243fbe1c7ccc6f60f`.
  Packaged executable SHA-256:
  `bbccde512fdcdaf58aefbc334706083a54a9d2f27e82d843313390baff63cfd2`.

The loopback fixture tests, native WindowServer captures and DiskImages helper
required execution outside the restricted sandbox. They did not connect to a
live account/model, install the app or change production data. Capture and
package verification both record clean source `b578520`.

## Remaining evidence limits

This is UI implementation and native fixture rendering evidence. It is not
measured usability, native screen-reader/assistive-technology evidence or an
accessibility conformance assessment. Native Windows/Linux, real-provider runs,
installed release behavior and OS-level window captures were not rerun for this
slice. The macOS package uses an ad-hoc development signature; it is not publicly
notarized. No installation, push or publication was performed.

No dependency, protocol, backend authority, schema, version, provider installation
or implicit execution change is included. Prepare and explicit Start, exact
file hashes/revisions and frozen external-action approvals remain separate.
