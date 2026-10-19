# Product crew design

Updated: 2026-10-19

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0046](../plans/0046-product-crew-design.md).

## Product surface

The owner rejected Task0045's visual result. The new design replaces its nested
navigation and text roster with a cohesive native work surface:

- One Project header contains the project picker and a segmented Chat / Crew /
  Outputs switch. The global entry is Your studio. Studio/Teams use a compact
  72px navigation rail below 1000px, with accessible route names/tooltips.
- Crew leads with a short heading and readable goal summary. Actual members
  appear as responsive role cards with distinct icons/accents and localized
  capability descriptions. Review has a quality-review description rather than
  presenting its underlying document ability as the user's objective.
- Exact member names remain owner/model data. Full technical instructions remain
  inspectable under Plan details. The surface does not invent agent activity,
  execution status or dependency links.
- A framed preparation area immediately follows the short plan, with a clear
  explanation and one primary action. Long details/rosters still scroll within
  a bounded body while Prepare remains reachable. Prepare and Start are separate.
- A contextual picker retains Plan, Direction, Recipes, Schedules and Activity
  without showing five secondary tabs on the default plan. Output Studio uses
  the same compact header and Files / Resources / Tools / Apps contextual picker.
- An empty Project conversation groups Mox, the real request composer and eight
  capability choices in one start surface. Conversations with history keep their
  bottom composer. Both use the same Project/ability TextEdit identity and command
  path; no separate submission implementation or data store was introduced.

## Interaction and scope

The twelve productive features remain available. Existing exact command/Project,
file path/hash/version, frozen approval, receipt and late-acknowledgment checks
remain in force. Private token input clears on view/context changes. The Host
remains the only execution authority; no core schema/provider/Cloud change.

Sidebar interactions have explicit route IDs shared across wide and rail layouts.
Actual resize/focus tests cover 1280→800→1280, then keyboard activation after
resizing. Context menus have bounded visible options and actual keyboard
selection tests. Plan content shrinking is limited to the Plan view; no long
request or disclosure can push its primary action out of the bounded window.

## Verification

- Final all-feature desktop suite: **160 passed, 0 failed, 0 ignored**.
- Focused Create/crew/output suite passed 54 tests; after the preparation-area
  adjustment, all 20 focused crew tests passed. Tests include TR/EN, both themes,
  800x560, actual context-menu interactions, three-role cards, private setup,
  frozen approval, saved goal adoption and exact Prepare-to-Start navigation.
- Added normal-window spacing and expanded-long-instructions tests check that
  the preparation area follows the plan and remains visible.
- Workspace/all-target/all-feature strict Clippy and workspace formatting pass.
- Independent source and actual native screenshot review passed. Existing action
  routes and scope guards remain; a responsive-sidebar focus risk was corrected
  and tested. Normal/minimum plan, chat, direction, output, setup and frozen
  approval surfaces were inspected. No remaining actionable blocker.
- **52 final native captures**: twelve scenes in TR/EN and light/dark at800x560,
  plus four normal-window captures. Local evidence is in the sibling output
  directory `branding/moxcrew/product-design-qa/native/qa.json`.
  Fixture binary SHA-256:
  `dc4574b39942afcf26a120e74f56fdcbb58f9ca9f1bfb37eafe897db7d3897e7`.
- Final source: `17b81e6`, following `a09eec7`, `2953ed1`, `1576bcf`, `ca4b088`.

## Development package

`dist/native/product-design/Moxcrew_0.12.0_product-design_aarch64.dmg`

Version remains0.12.0. The separate macOS ARM64 package has an ad-hoc signature,
not public notarization. License resources, signature, smoke-feature exclusion,
normal native launch/close, writer-lock release and DMG checksum verification
pass. Package evidence is in `dist/native/product-design/qa.json`.

- Executable:16,489,136bytes; SHA-256
  `6be23298d60a640cc09c1f8c7d83783bdc9a4ae6af27fd97b54b5fe5dd722466`.
- DMG:8,491,760bytes; SHA-256
  `f4aa77fa050d3eca6b787f88b99bc9dd110cee1132d32ab2cebd5d186fb2012a`.

Source and documentation are locally committed. No public release or push.
Windows/Linux, live provider inference and external account writes were not
validated in this native design correction.

No dependency, image asset, WebView, persistence schema, remote write, deployment
or push was added. Native QA uses isolated rendering fixtures without provider
inference. Existing runtime/tool/account/platform limits remain documented in
[productive Project crews](productive-project-crews.md).
