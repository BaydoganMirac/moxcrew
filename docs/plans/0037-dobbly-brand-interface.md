# Task 0037: Moxcrew native interface

Status: Complete; independently reviewed and locally verified

Updated: 2026-10-19

## Outcome and authority

The owner purchased moxcrew.com and explicitly requested the Moxcrew brand kit
and implementation in desktop and Cloud on 2026-10-18. Mox is the selected
concept-4 mascot; crew expresses the cooperating AI team. This approved card
covers the native presentation and packaging slice. It does not authorize a
hosted release or changes to production configuration.

Replace the current graphite/mint presentation with the Moxcrew identity across
the actual Rust/egui application. Keep the established local execution,
project scope, consent and receipt behavior. Shared visual decisions and the
display-name compatibility proposal are recorded in
[ADR 0037](../decisions/0037-dobbly-display-identity.md).

## Current implementation

- `crates/patchfleet-desktop/src/studio.rs`: bundled Manrope, dark-only palette,
  widget defaults and the three-stroke fleet mark.
- `crates/patchfleet-desktop/src/ui.rs`: eight tabs, navigation, forms, notices,
  account, source/decision/publication flows and UI boundary tests. Several
  fills and text colors remain literal values outside `studio.rs`.
- `crates/patchfleet-desktop/src/main.rs`: window identity, app icon, native
  startup, production error screen and feature-gated disposable smoke scenes.
- `crates/patchfleet-desktop/src/i18n.rs`: Turkish/English presentation copy.
- `crates/patchfleet-desktop/assets/icons`, `scripts/build-studio-icons.swift`, `Packager.toml`: native
  icon exports and packaged display identity.

The old Next.js Local UI is a legacy parity reference, not this task's app.
Task 0036 and its screenshots are the before-state, not Moxcrew evidence.

## Design contract

| Area | Decision |
|---|---|
| Character | Cobalt rectangular helper, large left eye, small orange-ringed right eye, orange top tab; no new mascot concept |
| Brand palette | Cobalt `#2854F4`, orange `#FF7A2F`, ink `#15223D`, paper `#F8F7F3`, cream `#FFF8E7` |
| Dark palette | Canvas `#101827`, surface `#182438`, text `#F7F8FC`, primary `#8DA6FF` |
| Typography | Existing local Manrope 450/650; UI body 14/20, secondary text normally 12/16; owner content retains its original text |
| Layout | 4 px rhythm; 8/12/16/24 px radii by role; one primary action per area |
| Controls | Main actions 44 logical px; dense desktop secondary controls may be 36 px; wrap or stack before clipping |
| Theme | Follow the OS light/dark preference, with deterministic light fallback; both palettes are implemented and tested |
| Motion | Static mascot, bounded 160-220 ms feedback only; no idle animation/repaint loop; keep transitions off if reduced motion cannot be honored reliably |

Use the flat vector symbol in chrome and the tactile mascot only in welcoming
or substantial empty states. Dense task, evidence and confirmation screens
remain quiet. Orange is a brand accent; it does not stand for every warning,
failure or completion.

No new persisted theme preference is introduced. The existing protected
`desktop-preferences.json` language schema stays intact. A later manual theme
preference would require its own bounded preference change, not a direct UI
write to the current strict schema.

## Screen changes

| Screen | Planned experience | Behavior that must survive |
|---|---|---|
| First use | Mascot, one clear first-project action, concise explanation of a project and local folder; provider setup help in plain language | Optional account, real native folder selection/preflight, actual provider requirements |
| Projects | Compact useful project cards, actual folder/source/decision counts, primary create action, secondary folder registration, scoped task/memory/decision shortcuts | Stable project/worktree links, source history, existing draft and folder association |
| Tasks / Work | Display label Tasks / Görevler, readable queue/detail/composer hierarchy, explicit working/waiting/review/failure states | Existing Work enum/action/payload IDs, draft preservation, supported Codex actions and actual receipts |
| Teams | Goal/workspace first, roles and limits second; stack the two columns at minimum width; separate question and approval cards | Inspectable packs, provider capability gates, concurrency/retry/time limits, revision checks |
| Agents | Consistent inspectable catalog cards with permissions and limits; restrained icon treatment | Custom pack text and actual install/selection behavior |
| Memory | Clear project scope, source list/detail hierarchy and discoverable drafting settings | Revision history, evidence identity, opt-in provider disclosure/usage consent, no automatic consent |
| Decisions | Distinguish proposed, accepted, rejected and superseded records; readable evidence and accept controls | Current citations/revisions, immutable accepted text, explicit replacement and owner acceptance |
| Providers | Friendly heading/help and explicit availability/capability rows; technical diagnostics remain accessible | Observation-only providers never acquire fake start/pause/resume controls |
| Cloud / Account | Moxcrew Cloud identity, clearly optional sign-in, readable connection/account/shared-memory states | Browser auth, matching code, paid/capability gates, separate project linkage/publication consent, stale downloaded copies |
| Loading/error/notice | Shared inline feedback with next action and localized text; errors never celebrate | Retry identity, busy/action guards, safe diagnostic redaction and data-recovery instructions |

Keep Workspace / Knowledge / System grouping. The first tab remains Projects;
do not introduce an unsupported Overview screen or a new global chat surface.
Account and language controls remain reachable in short windows, with scrolling
reserved for navigation/content rather than trapping the footer.

## Implementation cards and sequence

### N0 — Baseline and portable brand inputs

1. Capture current test/build results and normal/minimum screenshots using
   disposable data; preserve them separately from after-state evidence.
2. Promote the selected kit into versioned repository assets. Keep canonical
   public brand inputs under `design/moxcrew/`: tokens, approved vector marks,
   mascot, usage notes, font/license references and export provenance.
3. Check in native-ready assets and generated palette data. Cloud consumes its
   own checked-in snapshot of the same version; neither product requires a
   sibling checkout, network fetch or asset exporter to run/build.
4. Record the intended final changed-file boundary before editing the large
   `ui.rs`. Do not reorganize runtime/domain code for this presentation task.

Exit: palette/asset version is explicit; no machine paths in the public repo;
the desktop still builds with its normal Rust toolchain.

### N1 — Semantic theme, assets and native shell

1. Replace dark-only constants with a small semantic palette for two actual
   themes. Migrate shared widgets and remaining literal brand/status colors.
2. Install both egui styles without reinstalling fonts every frame. Follow the
   OS preference and update when it changes; retain an accessible light fallback.
3. Replace the fleet mark with the approved Moxcrew geometry. Cache any decoded
   mascot texture once using the existing PNG decoding path; add no image/SVG
   framework and no render-thread I/O.
4. Rework navigation/header/footer spacing and the primary/secondary/danger,
   focus, selected, disabled, field, disclosure and notice treatments.
5. Change visible window/chrome name to Moxcrew. Preserve `com.patchfleet.desktop`,
   crate/executable names, environment variables and private data locations.

Exit: Projects and first-use work in both languages/themes at 1280x880 and
800x560. No clipped footer/control or extra idle repaint loop.

### N2 — Work and supervision screens

Apply the screen contract to Projects, Tasks, Teams, Agents and Providers.
Replace jargon in helper copy without concealing prerequisites or adding
capabilities. Reuse existing action/form helpers and test IDs. Keep meaningful
labels alongside icons; keep draft text stable through theme/navigation changes.

Exit: create/edit/select, provider gating, question/approval and cancellation
controls still generate the same scoped commands and truthful states.

### N3 — Knowledge, account and exceptional states

Apply the contract to Memory, Decisions, shared-memory/publication/account
panels, startup errors and inline notices. Translate changed built-in copy in
both languages. Use the same controls/spacing; do not give these complex forms
independent visual rules. Inspect the actual long-text and stale/revision/error
cases, not only an empty happy-path screen.

Exit: every tab and consent/acceptance/retry boundary remains usable at the
minimum window, including Turkish labels and long owner text.

### N4 — Packaging, regression evidence and handoff

1. Export PNG/ICO/ICNS assets from the selected source, update the current icon
   build script so it cannot silently regenerate the obsolete fleet mark.
2. Update `Packager.toml` display product name/description and user-visible
   package notes. Preserve bundle identifier, executable and uninstall/data rules.
3. Produce a feature-off development package; launch only against fresh data,
   verify visible name/icon, close normally and verify writer release.
4. Record actual startup, idle CPU/RSS, asset and package-size deltas against
   a before-run made the same way. Investigate a material regression before
   accepting it; historical 0.12 numbers are context, not new measurements.
5. Update state, screenshot evidence and local commit. No installed-app
   replacement, signing/notarization claim or publication is implied.

## Verification matrix

- `cargo fmt --all -- --check`
- `cargo test --locked --workspace --all-features`
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
- `cargo build --locked --release -p patchfleet-desktop` with smoke feature off.
- Existing UI tests for all tabs, minimum control visibility, navigation footer,
  current scope/revision, unsent drafts, consent, capability gates and receipts.
- Add focused checks only for new theme/semantic-status selection and a changed
  interaction that existing checks do not cover. Do not snapshot every color.
- Actual native frames: light/dark, TR/EN, normal/minimum windows, first use,
  populated Projects/Tasks/Teams, Memory/Decisions and optional account states.
  Use existing `ui-smoke` infrastructure with disposable Host events.
- Text contrast target 4.5:1 for ordinary text; meaningful control/icon/focus
  boundaries 3:1. Measure actual component combinations, not just swatches.
- Keyboard activation/focus, scrolling, text wrapping and 1x/2x rendering checks.
- Run the relevant legacy protocol/package tests if packaging assertions or
  public branding contracts change. The old Local web UI is not redesigned.

## Explicit boundary and completion

No provider/model calls, owner-data tests, remote shell, event/schema changes,
language-preference migration, new Cloud payload, account requirement or change
to inference/billing entitlements. No replacement of user-entered aliases/text
or blind global rename of `patchfleet` identifiers.

Done means the screen matrix and behavior checks pass, actual after-state
evidence and platform limits are recorded, the feature-off local package is
verified and intended changes are committed locally. Windows/Linux runtime,
public signing and release must be reported accurately when not exercised.

Handoff: [native state](../state/dobbly-brand-interface.md).

## Implementation handoff

N0-N4 are implemented. Fresh checks, artifacts, measurements, screenshots and
platform limits are recorded in the [state](../state/dobbly-brand-interface.md).
The owner-selected Moxcrew/Mox identity supersedes the earlier Dobbly name; old
document filenames remain stable for existing links.
