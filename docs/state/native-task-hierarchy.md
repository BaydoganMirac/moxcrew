# Native task hierarchy

Updated: 2026-10-19

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0045](../plans/0045-native-task-hierarchy.md).

The owner rejected this milestone's visual result. [Task0046 product design](product-crew-design.md)
supersedes its layout; this report retains the interaction and package evidence
for the earlier milestone.

## User-visible result

Each Project has quiet Chat / Crew / Outputs navigation inside the native shell.
Productive Crew and Output Studio no longer cover the application with large
modal control panels. Bounded roster, account and mission inspectors retain
explicit actions. All twelve Task0044 capabilities remain available.

Crew leads with a short heading, wrapped goal, compact agent roster and one
pinned next action. Planning requests, provider overrides, budgets, provenance
and instructions open as secondary details. Outputs gives file previews the
main content area and keeps preview/export available. Disconnected apps lead
with connection setup; prepared operations replace the editable form with the
exact frozen preview and explicit approval.

The Chat composer has one accent action and quiet secondary controls. The
welcome mascot and eight capability choices use less space. Compact windows
reserve more room for content, including Direction and Recipe input fields.
Activity uses the actual scoped mission title; internal identity stays in details.

## Interaction and safety

Unsent text, selected file versions, pending receipts and exact prepared actions
survive ordinary view switching. Credential input clears when leaving its view.
Project removal/switch and Host epoch changes still invalidate transient scope.
No core event, provider, Cloud or production-tool contract changes.

A matching successful Prepare acknowledgment opens the selected mission and
Start crew only while the owner is still viewing that unchanged plan. Late
receipts never pull the owner out of another view. Opening a saved model plan
preserves an unsent goal and exposes explicit Use planned goal when needed.
Direction still requires actual affected-task selection and shows its replacement
warning. App execution still requires the frozen action identity and approval.

## Verification

- Final all-feature desktop suite: **158 passed, 0 failed, 0 ignored**.
- Focused Create/crew/output suite: **53 passed**. Tests use actual pointer and
  keyboard interactions in TR/EN, both themes and 800x560 windows. They cover
  draft-preserving navigation, long titles, pinned next actions, exact
  Prepare-to-Start routing, saved-plan goal adoption, disconnected app setup,
  inactive-view receipt matching, private token clearing and explicit approval.
- Workspace/all-target/all-feature strict Clippy and workspace formatting pass.
- Independent source and native screenshot review closed all three concrete
  route/setup findings; no remaining blocker.
- **52 final native captures**: twelve scenes across TR/EN and light/dark at
  800x560, plus four normal-window captures. Evidence is in the local sibling
  output directory `branding/moxcrew/task-hierarchy-qa/native/qa.json`. Fixture
  binary SHA-256: `05f791aac1a6c2c13edf398668f60f8edb674a34df78bed14ae0ff29d52b38bf`.
- The final source is `f1774a1`, following `1e11673`, `ed83c73` and `4d670a6`.

## Development package

`dist/native/task-hierarchy/Moxcrew_0.12.0_hierarchy_aarch64.dmg`

The separate package retains source version 0.12.0 and has an ad-hoc signature;
it is not a notarized public release. Smoke fixtures are absent from the release
binary. Signature/license resources, normal macOS application launch and close,
writer-lock release and DMG checksum verification pass. Evidence is in
`dist/native/task-hierarchy/qa.json`.

- DMG: 8,457,788 bytes; SHA-256
  `6aac0a9b7c4d1e2b2373c75d29a83b4a91d057cb578b8e083c1f4ce13990c1f5`.
- Executable: 16,423,264 bytes; SHA-256
  `2f5663b53aad159ace25a46766c6a024a14416f03bad2ed55cf0fdd6e06f7953`.

Implementation and documentation are committed locally. No push, public release
or live account operation is part of this correction.

No new dependency or persistence schema. Task0044's core/production evidence and
runtime limits remain in [productive Project crews](productive-project-crews.md).
Native fixtures prove rendering and interaction, not live model inference or
external account writes. Windows/Linux and public signing/notarization remain
outside this local macOS development verification.
