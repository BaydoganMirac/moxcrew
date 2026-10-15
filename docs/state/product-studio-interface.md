# Product studio interface

Status: Complete; independently reviewed and locally verified

Updated: 2026-10-15

Owner-approved [Task0036](../plans/0036-product-studio-interface.md) redesigns
presentation while preserving the Local runtime, events, providers and trust
boundary.

## Implemented

- Graphite/sage/mint studio theme, shared fleet mark, native platform icons and
  locally bundled Manrope 450/650 fonts (104,412 bytes total; SIL OFL 1.1).
- Grouped Workspace/Knowledge/System navigation, breadcrumb context and focused
  typography; input-driven hover feedback has no continuous animation loop.
- Actual project/folder/accepted counts, adaptive project cards, scoped decision
  and task shortcuts, native folder pickers and first-project onboarding.
- Unsent task content and its folder remain together when opening an existing
  draft. New empty drafts inherit an explicitly linked current folder only.
- Two-column goal/role team builder and adaptive inspectable agent catalog.
  Account/shared memory use columns when space permits. Long titles truncate
  with full-text hover; original owner content is never translated.
- Decision automation configuration is reached in Memory through Drafting
  settings; this resets consent. Narrow decision views disclose search/filters
  separately and keep owner acceptance visible. Source/generator evidence and
  canonical revision/consent gates remain unchanged.
- Sidebar scrolling reserves the account/language footer and reveals the active
  section after navigation without trapping manual scrolling.

## Verification

- 263 all-feature Rust checks passed, including 88 desktop checks. One new test
  covers project shortcut scope and unsent draft preservation. Four existing
  manual timing/provider checks remain ignored. Latest UI checks use actual
  bundled fonts; all 66 UI checks pass.
- The minimum-window test now requires the entire accept control to fit and
  verifies navigation against both the account and language controls.
- Strict all-target/all-feature Clippy and formatting pass after theme repair.
- Disposable actual native GL frames exercised TR/EN projects, memory, work,
  team setup, agent catalog, decision and first-use minimum windows. Final
  screenshots and package evidence are recorded below. No live model/provider
  turn or owner data was used.

## Limits

Native Windows/Linux runtime evidence and public signing/notarization remain
separate release work. The populated/minimum scenes are GL readbacks; the separate production image below is an actual macOS
WindowServer capture. Cloud deployment is outside this change.

## Reviewed release

Independent review accepted native `dce9036` without actionable findings. Final
263 all-feature Rust checks, strict all-target/all-feature Clippy, formatting
and 149 legacy protocol/package checks pass. The first legacy adapter timing
check failed during concurrent builds; the complete isolated rerun passed 149/149.
No unrelated timeout or runtime behavior was changed.

The feature-off 0.12 App passed ad-hoc signature, bundle identity/version, all
five folder-purpose keys and absence of test-only smoke controls. An explicit
NSWorkspace launch used a fresh empty data directory and failing provider stubs.
Its own bundle/PID and layer-zero window matched the packaged App. The actual
[production macOS window](../screenshots/studio-native-production-0.12-tr.png)
was captured successfully; normal termination exited and released the writer.

At eight seconds the process sampled 111,808 KiB RSS (109.2 MiB) and 0.16 seconds
cumulative CPU, including startup. This is an empty startup sample, not a
sustained or active-provider workload guarantee; history remains proportional
to canonical retained state. No production dependency or background service
was added. Bundled font data totals104,412 bytes.

`dist/native/Patchfleet_0.12.0_aarch64.dmg`: 6,972,449 bytes (6.65 MiB), SHA-256
`4e645b7bc3013c9bf877d24cf9a3a9c7bf2d63126466992b1724a640694b19be`.
The signed App executable is 13,557,312 bytes, SHA-256
`6275246477ec6f79b355b9abd8a73fe08a343c8b08c9393b23bc0c1bc1995794`.
DMG integrity passes; the image includes Apache LICENSE and FONT-LICENSE.txt.
Production signing/notarization, Windows/Linux runtime and deployment remain
operator release checks. The installed application and owner data were untouched.

Native frame evidence uses real disposable Host events, without provider turns:

- [Turkish projects](../screenshots/studio-native-projects-0.12-tr.png)
- [English projects](../screenshots/studio-native-projects-0.12-en.png)
- [English memory](../screenshots/studio-native-memory-0.12-en.png)
- [English team setup](../screenshots/studio-native-teams-0.12-en.png)
- [Minimum Turkish decisions](../screenshots/studio-native-decisions-0.12-tr-min.png)
- [Minimum first-use setup](../screenshots/studio-native-empty-0.12-tr-min.png)
