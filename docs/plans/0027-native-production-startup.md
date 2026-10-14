# Task 0027 Native production startup repair

Status: Verified — independent review pending

Owner direction: 2026-10-14, screenshot of an entirely white installed app.

## Outcome

The packaged production desktop displays its native loading/console/error view
on startup. Validate the shipping startup path, not just the screenshot-enabled
test build. Preserve project/work/team data, locale and the single writer.

## Diagnosis and resolution

- The installed and packaged 0.5.0 executables have the same SHA-256:
  `557af69a4f2183a370277efc2d54c856b57fd33f4973c19843f3f4c98bdfed0f`.
  The installed executable links native graphics frameworks and no WebKit.
- The already-mounted older DMG contains 0.3.0, which links WebKit. Launching
  that App with disposable data reproduces the entirely white window.
- With only that disposable 0.3 instance running, requesting
  `/Applications/Patchfleet.app` through LaunchServices with its normal
  configuration returns the old instance's PID. The resulting OS window capture
  is still white. Both bundles use `com.patchfleet.desktop`.
- After closing the test instances, a normal LaunchServices request for the
  installed 0.5 App with a fresh disposable directory displays the native
  Projects page, version 0.5.0 and language controls. Capture was taken after
  three seconds; this is presentation proof, not a first-frame timing benchmark.
- Recovery is to quit all old copies, eject the older image and open the
  installed App again. No data deletion, renderer change or new version is
  needed. The mounted owner image and installed App were not modified; only
  test-owned processes were terminated.

The owner's historical screenshot has no process identity. The tests establish
this matching failure and launch-routing mechanism; they do not assert a PID for
that historical window. No defect in the native production renderer was
reproduced. Temporary startup trace edits were removed completely.

The earlier `ui-smoke` images establish GL framebuffer rendering, not compositor
presentation. Release checks now explicitly include the actual production
window. Production evidence and the matching old-window reproduction are saved
in `docs/screenshots/native-production-0.5-tr.png` and
`docs/screenshots/native-legacy-0.3-blank.png`. Failing test provider executables
prevented old probes from reaching real sessions; no jobs, transcripts or Cloud
pairing were exercised.

## Validation

- All 24 desktop tests pass, including fresh-host failure/close and sparse idle
  polling. Canonical runtime and bridge code are unchanged.
- Production App ad hoc signature and the existing 0.5.0 DMG integrity pass.
  Its SHA-256 remains
  `c695ce7f7a56bbcb00c20a3919f9f08c7feead67c3f17060491c2ceb8e74bf83`.
- Formatting, strict workspace/all-target/all-feature Clippy and the restored
  production release build pass. Independent scope review is pending.
  No public release or owner-state migration.

## Scope and ownership

- Coordinator: diagnosis, this card/state, desktop `main.rs`/`bridge.rs`, minimal
  startup fix, regression checks and fresh development packaging.
- Reviewer: read-only root cause/acceptance check of the stable fix.
- A bounded diagnostic reader may inspect cached eframe/glutin/winit code only;
  it must not change dependencies, owner applications or data.

Use disposable data for native execution. Do not run owner jobs, pair Cloud,
read transcripts, reset preferences or delete canonical data. Inspect installed
version/hash and safe process/startup diagnostics to distinguish the shipped
binary from the already-superseded WebView. No push or publication is authorized.

## Acceptance

- Reproduce or conclusively identify the production blank-window cause.
- Production startup draws console/loading/error views without blocking on host
  open/provider discovery. Initial repaint is bounded; idle remains event-driven.
- Diagnostics and checks do not require private source/credentials or owner-data
  mutation. Production package and smoke proof exercise equivalent startup.
- Relevant Rust tests, fmt and strict Clippy pass; preserve close/writer/receipt
  and locale contracts. Record platform/proof limits honestly.
- Produce a verified macOS ARM64 App/DMG and local reviewable commit.
