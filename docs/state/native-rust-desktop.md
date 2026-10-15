# Native Rust desktop state

Status: 0.9.0 implemented and independently accepted; platform release checks pending

Updated: 2026-10-15

## Scope and architecture

The owner explicitly selected a WebView-free interface, then delegated the
technology choice with low RAM, responsiveness and lightweight distribution as
the main criteria. The native interface uses egui/eframe 0.36.2 and one Glow
renderer. Rust core, canonical events, existing data directory and Cloud
protocol remain intact. Tauri, React/Vite assets and their default build paths
were removed. Node modules remain only legacy behavior references.

The bounded async bridge handles owner commands off the render thread and
preserves their original identities/time windows for retries. Opening failures
appear in the window. Close immediately prevents new host work and then waits
for cleanup; a provider prepare cannot proceed to turn start during closing.
The exclusive writer remains owned until shutdown completes.

The 0.5.1 Projects view uses compact project cards, an explicit native folder
setup action and collapsed technical identity. Registration and one-off work
paths are read-only previews selected through RFD's platform dialog. A bounded
preflight and Host revalidation distinguish missing, unsafe, non-Git, denied
and read-only folders. OS selection/settings guidance does not change ACLs or
request Full Disk Access. AppKit folder purpose keys are bundled. Existing
canonical registration receipts survive later access loss/deletion through the
unchanged intent equality check; new commands still validate current access.

The picker is created on the UI thread and awaited on the existing runtime.
App logic consumes background results and close signaling even while UI drawing
is occluded. Cancel preserves the draft and writes no owner command. Success,
warning and error feedback are localized, dismissible and expose only safe
technical codes; folder recovery starts a new selection instead of retrying an
old path. Confirmation footers stay outside the scroll body and paths truncate
with a complete tooltip. See [ADR 0028](../decisions/0028-native-folder-consent-and-feedback.md).

Eight native sections cover stable projects, source memory, decisions, work, teams, packs,
providers and Cloud. Turkish/English controls switch through the owner Host and
persist in protected display configuration; owner source and custom text stay raw.
Forms preserve revisions, validate limits, mask pairing codes and restrict
controls to proven capabilities/owned runs. Damaged Cloud state shows explicit
private-file recovery guidance with no unavailable reset/pair controls.

Idle rendering responds to input/state events, with animations disabled.
Provider probes run at most once a minute or on explicit refresh. Active owned
signals and scheduling update every five seconds; idle ticks are sparse.
Unpaired Cloud returns before copying projections and canonical history.

## 0.9 optional account

[Task0033](../plans/0033-desktop-account-sign-in.md) adds native TR/EN account
sign-in through the system browser, strict loopback/S256 approval, fresh account
and subscription state, cancellation and installation sign-out. The Cloud server
origin is configurable until a live service exists. Local still requires no
account. The existing protected Cloud credential schema remains unchanged;
provider browser sessions and account overlays are not durable Local facts.

The account lane keeps Local owner work responsive and preserves its original
receipts/retries. Independent review's stale-snapshot race is fixed and covered
by a real callback/HTTP regression. Final202 Rust tests,149 legacy checks,
strict workspace Clippy and formatting pass. Five actual native account frames
were inspected. The feature-off App/DMG, normal OS launch/close, short resource
sample and release limits are recorded in [account state](desktop-account.md).

## 0.7 explicit shared decisions

- [Task 0031](../plans/0031-cloud-memory-publication.md) adds optional subscription
  Cloud publication and exact same-workspace reader copies. The Cloud view
  explicitly creates/links a shared project; accepted Local ADRs require a
  preview and checked upload confirmation. Local acceptance remains free and
  does not publish automatically. Source bodies/origin events remain Local.
- The preview uses readable translated field labels with original owner text,
  a bounded scrolling body and separate consent/action controls. Opaque
  references and exact wire fields are collapsed technical details. Revision,
  project, role, connection and queue changes reset consent. Read-only downloaded
  copies have current/historical, offline/stale and verification labels.
- All 54 native regression tests pass, covering explicit upload clicks and
  allowlists, scope/revision/connection invalidation, raw owner text, bounded
  minimum-window controls and original owner-error retry protection.
- Actual disposable PostgreSQL/HTTP-backed native GL frames show
  [Turkish exact reader](../screenshots/native-shared-reader-0.7-tr.png),
  [English minimum reader](../screenshots/native-shared-reader-0.7-en-min.png) and
  [Turkish publication preview](../screenshots/native-publication-preview-0.7-tr.png).
  All three were inspected and exited normally. The preview's checkbox remains
  unchecked; this fixture does not upload that illustrated Local ADR. The
  reader body is downloaded from the real disposable service. These GL frames
  alone do not establish production OS presentation.
- No production dependency, vector engine or background database service was
  added to the desktop. Memory HTTP work has a three-second aggregate budget,
  a four-request tick limit and immediate close cancellation. Shared reader
  cache limits and canonical V5 rollback requirements are recorded in
  [memory state](project-memory.md) and [installation](../install.md).
- Final verification passes 158 Rust tests (104 core/54 desktop), 149 legacy
  tests, strict workspace Clippy and formatting. Independent review accepted
  core `46e149d`, queue-cap repair `bea89cd`, UI `1d44b55` and the matching
  separate memory transport after all three concrete findings were repaired.
- The final feature-off App built from coordinator integration `896c3bd`
  launched through NSWorkspace with copied disposable reader data and failing
  provider stubs. Its own bundle/PID and native layer-zero window were verified.
  Normal close exited and released the writer lock. OS capture returned “could
  not create image from window”; no final production pixel screenshot is claimed.
  This eight-second startup sample reached 106.63 MiB RSS and 0.17 seconds
  cumulative CPU, including startup and a disconnected fixture connection.
  It does not establish sustained or cross-platform resource use.

## 0.6 decision evidence

- [Task 0030](../plans/0030-local-decision-lifecycle.md) adds source-backed,
  immutable ADR drafts, explicit accept/reject and atomic replacement, exact
  original source evidence and history. No Cloud/account/model is required.
- 123 Rust tests pass: 81 core and 42 desktop checks, with one pre-existing
  manual large-log probe ignored. All 149 legacy tests and 246 original V1
  prefix hashes pass. Workspace formatting and strict all-target/all-feature
  Clippy pass. Core/Host/Bridge review accepted `9ab17bb`/`95ab7c9`; native UI
  implementation is `6aa36d7`.
- UI tests exercise current source citations, owner transition payloads,
  project/decision/source revision guards, failed-read retry, original owner
  text, successful draft reset and rejected draft preservation. Bounded Bridge
  pages are ephemeral and invalidated when their project manifest changes.
- Review fixes `f4bc387`, `5b88067` and `d652bf9` preserve later draft edits on
  original-command retry, and prevent automatic reads replacing an owner error
  or its exact pending command. Regression tests use an actual derived-cache
  write failure after durable append, plus UI edit/citation/epoch changes.
  Independent final code review accepted both fixes through `d652bf9` with no
  remaining concrete blocker. Package verification below was run again against
  the rebuilt feature-off App after those fixes.
- Real macOS GL captures show [Turkish](../screenshots/native-decisions-0.6-tr.png),
  [minimum Turkish](../screenshots/native-decisions-0.6-tr-min.png) and
  [minimum English](../screenshots/native-decisions-0.6-en-min.png) using four
  real Host-created illustrative ADRs in all lifecycle states. The eighth
  navigation row initially overlapped the language footer at 800×560; a bounded
  navigation scroll region and TR/EN clip assertions fix that collision.
  All scenes exit normally and use disposable sources only. Images are GL
  readbacks after sizing, not production OS presentation captures.
- The feature-off production App launched through NSWorkspace as a separate
  instance using copied disposable ADR data and failing provider stubs. Its own
  bundle path/PID and native layer-zero window were verified. Normal termination
  exited and released the writer lock. OS pixel capture again returned “could
  not create image from window”; no final production screenshot is claimed.
- This eight-second production sample used 106.02 MiB RSS and 0.14 seconds
  cumulative CPU including startup. It is not a sustained workload or
  cross-platform performance promise. No dependency, vector engine or background
  database service was added. Canonical cache/replay remains history-dependent.

## Earlier 0.5.1 evidence

- Rust workspace: 96 tests pass, including 31 native command/bridge/UI/localization
  tests. One manual large-log timing probe remains outside the ordinary suite.
- UI checks paint all sections with empty/populated snapshots and exercise
  registration, work revisions, busy/capability controls, owner decisions,
  previous-host run controls and damaged Cloud recovery.
- Independent review confirmed immediate close signaling, writer release,
  durable retry identities, safe errors, Cloud recovery, immutable project evidence,
  clock correction recovery and language/source boundaries.
- Task 0028 was independently accepted through `f155721`, covering new access
  diagnostics, canonical registration retries, neutral picker cancel/close,
  explicit project linking and idle recovery controls. No remaining concrete
  blocker was found; platform/presentation limits below remain explicit.
- A real native macOS ARM64 frame renders the UI and Rust snapshot. Timed
  normal close exits successfully using disposable data and no provider probes.
- Task 0027 established macOS window presentation for production 0.5.0: the
  then-installed App and packaged executable had identical SHA-256 hashes, and
  a normal LaunchServices launch with disposable data displays the native
  console in an OS window capture. The older screenshot-feature checks below
  read the GL framebuffer and did not establish this presentation evidence.
- For 0.5.1, an actual AppKit folder sheet opened with a localized title and
  OS controls. Timed close succeeded with the sheet open and with an actual
  read-only disposable Git folder; no owner registration was submitted. The
  permission fixture's mode was restored during cleanup. Final Turkish/English
  800×560 form captures show complete footer buttons, wrapped guidance and
  truncated paths; these final images are GL readbacks after sizing, not OS
  presentation captures. Headless tests also check the footer's visible clip.
- The 0.5.1 production App launched through NSWorkspace as a fresh instance with
  explicit disposable data and failing provider stubs. The returned bundle/PID
  matched the packaged App, a native window was listed, and normal termination
  released the process/writer. The final OS pixel capture returned “could not
  create image from window”; no successful final production screenshot is
  claimed. Earlier 0.5.1 sheet pixels were inspected locally, but are not committed
  because the OS sidebar contains machine-specific details.
- Earlier 0.5.0 Turkish and English frames were captured in 0.823 and 0.286 seconds,
  including disposable Host fixture creation, PNG capture and process exit; the
  second used warmed system caches. Final Turkish memory-scene idle RSS samples
  were 89.7–92.4 MiB. Earlier local samples ranged up to 107.5 MiB. Eight seconds
  of cumulative CPU sampling showed no increase (below roughly 0.13% of one core
  at ps timer precision). The test scene has one project and three messages,
  with provider probes disabled. RSS includes
  resident shared graphics/framework pages; window-server costs, large history,
  actual provider processes and other platforms are separate. These are measured
  local observations, not cross-platform performance promises.
- The final 0.5.1 production sample reached 105.2 MiB RSS and 0.14 seconds
  cumulative CPU over an eight-second cold launch, including startup and failing
  probe stubs. Separate sheet/error tests reached 105–106 MiB and exited normally.
  These short samples do not establish sustained idle/active-work performance.
- Legacy event/protocol reference: all 149 tests pass after the native package
  contract update.
- Runtime dependency tree contains egui/eframe/Glow and no Tauri/Wry/WebKit,
  WebView2 or WGPU renderer. The production binary does not require Node.

## Final local package

- `dist/native/Patchfleet.app` contains the production binary, without test-only
  screenshot/timed-close features. Version is 0.7.0 and the bundle identifier
  remains `com.patchfleet.desktop`.
- `dist/native/Patchfleet_0.7.0_aarch64.dmg` is 6,358,403 bytes (about 6.06 MiB).
  SHA-256: `0a070b9c5be12d9c4e0326dd09bfa8142694a9d1ef85ae4265ac446e15449ea9`.
- The signed executable is 12,348,496 bytes, SHA-256
  `e905068e20a88cf2ba2260d02c077ef188f1593fadd2d22674c9690ba898409b`.
  Its linked frameworks are native system
  graphics/window libraries; WebKit is absent.
- DMG integrity, App ad hoc signature, all five folder-access purpose keys and
  bundle version/identifier checks pass. Test-only scene/capture controls are
  absent from the production binary. Production hook ingress was verified in
  Task 0027; Task 0031 does not change it.
- cargo-packager produced the App. The local DMG was created with hdiutil using
  a distinct versioned volume name to preserve the owner's already-mounted old
  Patchfleet image. No installed application, owner state, branch push, public
  upload or live-data migration was performed.

## Visual follow-up

The subsequent entirely-white-window report was reproduced with the mounted
0.3 WebView App. While that test instance was running, a normal LaunchServices
request for the installed 0.5 App returned the old instance's PID and displayed
the same white window. Closing that test instance and normally launching the
installed 0.5 App displayed the console. Both checks used disposable directories
and failing provider test commands; owner data and the mounted image were left
intact. No native renderer change or version bump was justified. See
[Task 0027](../plans/0027-native-production-startup.md), the
[production window](../screenshots/native-production-0.5-tr.png) and the
[reproduced old window](../screenshots/native-legacy-0.3-blank.png).

The owner rejected the first native appearance. The implemented redesign uses
quiet icon navigation, consistent typography/color tokens, padded inputs,
text-sized status chips, compact forms, responsive columns and clear first-project
onboarding. The actual final native frame is preserved in
[the original screenshot](../screenshots/native-desktop-0.4.png). Current
[Turkish](../screenshots/native-memory-0.5-tr.png) and
[English](../screenshots/native-memory-0.5-en.png) native frames show disposable
project memory entered through the actual Host. Fixture text explicitly identifies
its illustrative purpose; provider status is not fabricated as real work.
Earlier 0.5.1 [project setup](../screenshots/native-projects-0.5.1-tr.png),
[Turkish access form](../screenshots/native-folder-access-0.5.1-tr.png) and
[English access form](../screenshots/native-folder-access-0.5.1-en.png) use only
disposable illustrative data. The form images are minimum-window GL captures.

## Release limits

- Native CI targets macOS ARM64/Intel, Windows x64 and Linux x64/ARM64, but its
  unrun platform jobs are not local build/runtime evidence.
- Actual Windows/Linux folder dialogs and permission behavior, and global macOS
  TCC deny/grant settings were not exercised. The Windows directory-handle access
  helper was type-checked against cached MSVC metadata; a full cross-target build
  remains blocked by unavailable Windows SDK headers in the existing ring build.
  Linux requires a working desktop portal backend or Zenity; these are declared
  runtime prerequisites. Owner privacy settings were not altered for a test.
- Local development artifacts use ad hoc signing. Public signing/notarization
  and publication remain owner release actions.
- Live provider turns/cancels, staging Cloud pairing and owner-data migrations
  were not performed by the tests.
- Native egui renders its own widgets in an OS window; it does not use platform
  AppKit/WinUI widgets. Accessibility support is enabled.
- Memory still scales with retained canonical history and visible operational
  data. Source memory is implemented with compact metadata/scoped reads, but the
  canonical event cache remains proportional to history. Vector indexing,
  agent tools and automatic extraction remain later slices. Explicit Cloud
  publication is implemented in 0.7 and requires compatible service deployment.

See [ADR 0024](../decisions/0024-native-rust-desktop-ui.md),
[Task 0024](../plans/0024-native-rust-desktop-ui.md), [release](../release.md) and
[installation](../install.md).

## 0.8 retrieval increment

[Task0032 search state](project-memory-search.md) records explicit TR/EN Cloud
search, free Local ranked keyword retrieval, actual model/backend/native frame
proofs and remaining operator/platform limits. Model inference runs only on the
optional Cloud server; the native runtime/dependency set is unchanged.

The 0.8 development DMG is6,398,816bytes (6.10MiB). Its feature-off native App
passed signature/purpose-key/smoke-marker checks, explicit macOS launch, window
creation, normal close and writer-lock release. Sampled idle RSS105.9MiB;
model/runtime dependencies remain absent from desktop. Exact hashes and limits
are in the search state.
