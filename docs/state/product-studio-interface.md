# Product studio interface

Status: Implemented; independent review and release verification in progress

Updated: 2026-10-15

Owner-approved [Task0036](../plans/0036-product-studio-interface.md) redesigns
presentation while preserving the Local runtime, events, providers and trust
boundary.

## Implemented

- Graphite/sage/mint studio theme, shared fleet mark, native platform icons and
  locally bundled Manrope450/650 fonts (104412bytes total; SIL OFL1.1).
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

## Verification so far

- 263 all-feature Rust checks passed, including88 desktop checks. One new test
  covers project shortcut scope and unsent draft preservation. Four existing
  manual timing/provider checks remain ignored. Latest UI checks use actual
  bundled fonts; all66 UI checks pass.
- The minimum-window test now requires the entire accept control to fit and
  verifies navigation against both the account and language controls.
- Strict all-target/all-feature Clippy and formatting pass after theme repair.
- Disposable actual native GL frames exercised TR/EN projects, memory, work,
  team setup, agent catalog, decision and first-use minimum windows. Final
  screenshots, package hashes and measured resources will be recorded after
  review. No live model/provider turn or owner data was used.

## Limits

Native Windows/Linux runtime evidence and public signing/notarization remain
separate release work. GL screenshots establish rendered native frames, not
macOS WindowServer pixel capture. Cloud deployment is outside this change.
