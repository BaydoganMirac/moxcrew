# Native Rust desktop state

Status: Implemented and native render verified; platform release checks pending

Updated: 2026-10-14

## Scope and architecture

The owner explicitly selected a WebView-free interface, then delegated the
technology choice with low RAM, responsiveness and lightweight distribution as
the main criteria. The local 0.5 interface uses egui/eframe 0.36.2 and one Glow
renderer. Rust core, canonical events, existing data directory and Cloud
protocol remain intact. Tauri, React/Vite assets and their default build paths
were removed. Node modules remain only legacy behavior references.

The bounded async bridge handles owner commands off the render thread and
preserves their original identities/time windows for retries. Opening failures
appear in the window. Close immediately prevents new host work and then waits
for cleanup; a provider prepare cannot proceed to turn start during closing.
The exclusive writer remains owned until shutdown completes.

Seven native sections cover stable projects, source memory, work, teams, packs,
providers and Cloud. Turkish/English controls switch through the owner Host and
persist in protected display configuration; owner source and custom text stay raw.
Forms preserve revisions, validate limits, mask pairing codes and restrict
controls to proven capabilities/owned runs. Damaged Cloud state shows explicit
private-file recovery guidance with no unavailable reset/pair controls.

Idle rendering responds to input/state events, with animations disabled.
Provider probes run at most once a minute or on explicit refresh. Active owned
signals and scheduling update every five seconds; idle ticks are sparse.
Unpaired Cloud returns before copying projections and canonical history.

## Current evidence

- Rust workspace: 81 tests pass, including 24 native command/bridge/UI/localization
  tests. One manual large-log timing probe remains outside the ordinary suite.
- UI checks paint all sections with empty/populated snapshots and exercise
  registration, work revisions, busy/capability controls, owner decisions,
  previous-host run controls and damaged Cloud recovery.
- Independent review confirmed immediate close signaling, writer release,
  durable retry identities, safe errors, Cloud recovery, immutable project evidence,
  clock correction recovery and language/source boundaries.
- A real native macOS ARM64 frame renders the UI and Rust snapshot. Timed
  normal close exits successfully using disposable data and no provider probes.
- Production launch proof now also covers macOS window presentation: the
  installed 0.5 App and packaged executable have identical SHA-256 hashes, and
  a normal LaunchServices launch with disposable data displays the native
  console in an OS window capture. The older screenshot-feature checks below
  read the GL framebuffer and did not establish this presentation evidence.
- Actual 0.5 Turkish and English frames were captured in 0.823 and 0.286 seconds,
  including disposable Host fixture creation, PNG capture and process exit; the
  second used warmed system caches. Final Turkish memory-scene idle RSS samples
  were 89.7–92.4 MiB. Earlier local samples ranged up to 107.5 MiB. Eight seconds
  of cumulative CPU sampling showed no increase (below roughly 0.13% of one core
  at ps timer precision). The test scene has one project and three messages,
  with provider probes disabled. RSS includes
  resident shared graphics/framework pages; window-server costs, large history,
  actual provider processes and other platforms are separate. These are measured
  local observations, not cross-platform performance promises.
- Legacy event/protocol reference: all 149 tests pass after the native package
  contract update.
- Runtime dependency tree contains egui/eframe/Glow and no Tauri/Wry/WebKit,
  WebView2 or WGPU renderer. The production binary does not require Node.

## Final local package

- `dist/native/Patchfleet.app` contains the production binary, without test-only
  screenshot/timed-close features. Version is 0.5.0 and the bundle identifier
  remains `com.patchfleet.desktop`.
- `dist/native/Patchfleet_0.5.0_aarch64.dmg` is 5,844,693 bytes (about 5.57 MiB).
  SHA-256: `c695ce7f7a56bbcb00c20a3919f9f08c7feead67c3f17060491c2ceb8e74bf83`.
- The executable is 11,586,624 bytes. Its linked frameworks are native system
  graphics/window libraries; WebKit is absent.
- DMG integrity, App ad hoc signature and bundle version/identifier checks pass.
  Final production native hook ingress also passes with five sanitized fields
  and no canonical writer, window or provider process.
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

## Release limits

- Native CI targets macOS ARM64/Intel, Windows x64 and Linux x64/ARM64, but its
  unrun platform jobs are not local build/runtime evidence.
- Local development artifacts use ad hoc signing. Public signing/notarization
  and publication remain owner release actions.
- Live provider turns/cancels, staging Cloud pairing and owner-data migrations
  were not performed by the tests.
- Native egui renders its own widgets in an OS window; it does not use platform
  AppKit/WinUI widgets. Accessibility support is enabled.
- Memory still scales with retained canonical history and visible operational
  data. Source memory is implemented with compact metadata/scoped reads, but the
  canonical event cache remains proportional to history. Decision lifecycle,
  vector indexing, agent tools and automatic extraction remain later slices.

See [ADR 0024](../decisions/0024-native-rust-desktop-ui.md),
[Task 0024](../plans/0024-native-rust-desktop-ui.md), [release](../release.md) and
[installation](../install.md).
