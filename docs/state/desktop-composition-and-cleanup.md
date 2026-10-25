# Desktop composition and generated artifact cleanup

Updated: 2026-10-25

Status: Implemented and locally verified; owner visual acceptance not asserted

Plan: [Task0054](../plans/0054-desktop-composition-and-build-cleanup.md).
Components: [UI kit](../../design/moxcrew/UI-KIT.md).

## Owner correction

The owner rejected the prior visual result and clarified that layout, rather
than animation, is the problem. Task0053's technical acceptance did not establish
owner design acceptance. This slice changes the actual native composition.

## Cleanup

- Removed reproducible Cargo artifacts with `cargo clean`: the tool reported
  327,223 files / 72.5 GiB logical total. The initial on-disk `du` measurement was
  46 GiB; these measurements differ because logical and allocated sizes differ.
- Removed 31 obsolete paths under `dist/native`, including old App/DMG versions
  and the preceding Task0053 DMG. Retained only the existing
  `dist/native/ux-rules/Moxcrew.app` fallback (~17 MiB).
- Filesystem free space reported by `df -h`: 5.7 GiB before, approximately 51 GiB
  after cleanup and final verification. This is actual filesystem availability,
  not the sum of logical file lengths.
- Rebuilt only development/test artifacts with `CARGO_INCREMENTAL=0`,
  `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=4`.
  Final `target` footprint is approximately 1.1 GiB. No release build, App bundle,
  DMG, installation, dependency removal or provider setup was performed.
- Source, toolchains, dependency downloads, Project data, brand master assets and
  prior screenshot evidence were retained. The fallback App predates this UI
  change; it must not be presented as the updated installed product.

## Implemented composition

- Normal navigation remains 208 points wide when Crew/Outputs opens. A 64-point
  rail serves narrow chat/team windows; its 8-point inset leaves room for the
  language code on one line. Other narrow form pages retain their full navigation.
- A 760-point conversation and adaptive 420-point inspector have distinct roles.
  A 16-point gutter separates work/composer surfaces from the inspector. Wide
  context uses the existing view tabs; only narrow detail surfaces repeat Back.
- Welcome heading, editor and starting suggestions share a left reading axis.
  Connection/status metadata is below the composer surface. The editor's bounded
  text height, direct Attach and trailing validated Send/Prepare are preserved.
- Regular/compact desktop controls use 36/32 points, with 14-point body text and
  32-point chip/removal targets retained. Shared radii are 8/12. Fewer decorative
  borders and no ambient canvas bloom or composer glow; no animation was added.
- Goal, roles, Prepare/Edit and explanation form one crew proposal. Roster rows
  and preparation guidance avoid nested cards. Action rows are vertically bounded
  before their guidance, so expanding long details cannot push the footer out.
- Output search and current type share a toolbar. The bounded type picker retains
  every file-type chip. Duplicate filename headings were removed; package actions
  use a flat divider/footer instead of a second raised card.
- Mission status cards expose their exact goal through a keyboard-openable
  disclosure; they no longer synthesize a second owner message. Real conversation
  turns are unchanged, including different requests with identical text.
- Global context content IDs preserve search focus across wide/narrow layouts
  even though the Back control is only rendered in the narrow layout.

Existing Project/provider/receipt and exact file revision/hash checks, explicit
Prepare then Start and external-action approvals remain unchanged. No capability,
backend contract, schema, dependency or product version change is included.

## Source and verification

Runtime commits: `8897c6c` (crew/output composition), `3745030` (bounded crew
footer rows), `01482ff` (shell/composer/components), `ab2205c` (rail/gutter).
The final runtime checkpoint is `bfc250d` (mission goal disclosure); following
documentation commits do not change the executable. All workers used disjoint file ownership.

Local evidence: `../output/branding/moxcrew/composition-qa/` relative to the
repository root, including cleanup/package manifests and final test/build logs.

- **229 desktop tests pass**, zero failures, with all features. Existing draft,
  provider, receipt, Prepare/Start, keyboard and 100/125/150% scale tests remain.
  The proposal action containment assertion was added; filter keyboard tests now
  open the actual type picker. A new mission-goal keyboard test verifies that
  historical instructions remain inspectable without a synthetic owner bubble.
  No containment assertion was weakened.
- Strict workspace/all-target/all-feature Clippy passes with `-D warnings`;
  formatting, changed Markdown links, token JSON and whitespace checks pass.
- **17 final native fixture screenshots**, all processes exit 0:
  `native-verified/qa.json` records 15 cases (six TR/dark normal, six TR/dark minimum,
  plus EN/light minimum offline, busy and output); `crew-verified/qa.json` records
  two additional normal/minimum Crew inspector images. Both verified indexes
  record clean source `c0050ff` (runtime `bfc250d` plus documentation), the debug
  binary hash and individual image hashes.
- The first capture pass exposed a wrapped rail language code and the missing
  gutter between conversation and inspector. Both were corrected, tested and
  recaptured before handoff. The additional mission-goal correction was also
  keyboard-tested and recaptured. Earlier iteration directories retain their
  original source provenance.
- Native captures are framebuffer fixtures using disposable data, without live
  inference or provider/account probes. Loopback fixture tests and WindowServer
  captures required execution outside the restricted sandbox.

Independent read-only review of runtime `bfc250d` and all 17 verified images
found no remaining actionable blocker, including in the mission goal disclosure
and preserved real turns. The owner's evaluation of the visual result remains
authoritative; passing rendering and behavior tests is not a premium-design
acceptance claim.

## Limits

No installed release run, new release package, live provider execution,
Windows/Linux native review, screen-reader audit or moderated user study was
performed in this slice. The retained fallback App does not contain this new UI.
No push, publication or installation was performed.
