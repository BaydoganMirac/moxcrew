# Native Rust desktop state

Status: Implemented and native render verified; platform release checks pending

Updated: 2026-10-14

## Scope and architecture

The owner explicitly selected a WebView-free interface, then delegated the
technology choice with low RAM, responsiveness and lightweight distribution as
the main criteria. The local 0.4 interface uses egui/eframe 0.36.2 and one Glow
renderer. Rust core, canonical events, existing data directory and Cloud
protocol remain intact. Tauri, React/Vite assets and their default build paths
were removed. Node modules remain only legacy behavior references.

The bounded async bridge handles owner commands off the render thread and
preserves their original identities/time windows for retries. Opening failures
appear in the window. Close immediately prevents new host work and then waits
for cleanup; a provider prepare cannot proceed to turn start during closing.
The exclusive writer remains owned until shutdown completes.

Six native sections cover projects, work, teams, packs, providers and Cloud.
Forms preserve revisions, validate limits, mask pairing codes and restrict
controls to proven capabilities/owned runs. Damaged Cloud state shows explicit
private-file recovery guidance with no unavailable reset/pair controls.

Idle rendering responds to input/state events, with animations disabled.
Provider probes run at most once a minute or on explicit refresh. Active owned
signals and scheduling update every five seconds; idle ticks are sparse.
Unpaired Cloud returns before copying projections and canonical history.

## Current evidence

- Rust workspace: 52 tests pass, including eleven native command/bridge/UI
  tests. One manual large-log timing probe remains outside the ordinary suite.
- UI checks paint all sections with empty/populated snapshots and exercise
  registration, work revisions, busy/capability controls, owner decisions,
  previous-host run controls and damaged Cloud recovery.
- Independent review confirmed immediate close signaling, writer release,
  durable retry identities, safe errors and Cloud recovery controls.
- A real native macOS ARM64 frame renders the UI and Rust snapshot. Timed
  normal close exits successfully using disposable data and no provider probes.
- The final actual native frame was captured in 0.690 seconds, including PNG
  capture and process exit. Final fresh-data idle RSS was 107.27 MiB; earlier
  local samples were around 85.6–85.8 MiB. Eight seconds of final cumulative CPU
  sampling reported 0.00% of one core at ps timer precision (below roughly 0.13%).
  The native test build used empty data and disabled provider probes. RSS includes
  resident shared graphics/framework pages; window-server costs, large history,
  actual provider processes and other platforms are separate. These are measured
  local observations, not cross-platform performance promises.
- Legacy event/protocol reference: all 149 tests pass after the native package
  contract update.
- Runtime dependency tree contains egui/eframe/Glow and no Tauri/Wry/WebKit,
  WebView2 or WGPU renderer. The production binary does not require Node.

## Final local package

- `dist/native/Patchfleet.app` contains the production binary, without test-only
  screenshot/timed-close features. Version is 0.4.0 and the bundle identifier
  remains `com.patchfleet.desktop`.
- `dist/native/Patchfleet_0.4.0_aarch64.dmg` is 5,681,609 bytes (about 5.42 MiB).
  SHA-256: `0188a842009c41d55a3d9267a24916eb120d4448f28db7a9cddb166b21bbf1af`.
- The executable is 11,239,760 bytes. Its linked frameworks are native system
  graphics/window libraries; WebKit is absent.
- DMG integrity, App ad hoc signature and bundle version/identifier checks pass.
  Final production native hook ingress also passes with five sanitized fields
  and no canonical writer, window or provider process.
- cargo-packager produced the App. The local DMG was created with hdiutil using
  a distinct versioned volume name to preserve the owner's already-mounted old
  Patchfleet image. No installed application, owner state, branch push, public
  upload or live-data migration was performed.

## Visual follow-up

The owner rejected the first native appearance. The implemented redesign uses
quiet icon navigation, consistent typography/color tokens, padded inputs,
text-sized status chips, compact forms, responsive columns and clear first-project
onboarding. The actual final native frame is preserved in
[the screenshot](../screenshots/native-desktop-0.4.png); it contains fresh test
data, with provider status honestly shown as not checked. No demo activity
is presented as real work.

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
  data. This migration does not replace the event cache or implement project
  memory/vector indexing.

See [ADR 0024](../decisions/0024-native-rust-desktop-ui.md),
[Task 0024](../plans/0024-native-rust-desktop-ui.md), [release](../release.md) and
[installation](../install.md).
