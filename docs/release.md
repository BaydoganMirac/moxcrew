# Native desktop release

The 0.11 source builds a standalone Rust/egui desktop. Installers use
cargo-packager 0.11.8. Node, Tauri, WebView2 and WebKit are not runtime or desktop
build requirements. npm publication is retired. Pushing, publishing installers,
public signing and Cloud deployment remain explicit owner release actions.

## Verify a reviewed version

1. Run Rust workspace tests, Clippy and formatting checks.
2. Build the standalone release binary and package it through `Packager.toml`.
3. Verify an actual native frame, host-open errors, forms, normal close,
   provider loss/cancel and offline-copy migration on each advertised OS.
4. Inspect the native CI macOS ARM64/Intel, Windows x64 and Linux x64/ARM64
   artifacts. A configured matrix does not establish native success.
5. Measure startup, idle process memory/CPU and active-work behavior. Provider
   process costs and canonical history size must be distinguished from UI cost.
6. Test allowlist redaction and consented staging Cloud pairing. Keep package
   checksums and a pre-upgrade backup; do not claim an ad hoc development
   signature as a public signed release.

## Packaging

```sh
cargo install cargo-packager --version 0.11.8 --locked
cargo build --locked --release -p patchfleet-desktop
cargo packager --config Packager.toml --formats app,dmg
```

Local output is `dist/native`. Windows formats are `nsis,wix`; Linux uses
`deb,appimage`. CI stages one target executable for the pinned packager's
configured binaries directory, archives only installer outputs, and preserves
executable permissions. It does not publish a release or use signing secrets.

The native folder picker is pinned to RFD 0.17.2: AppKit on macOS, Shell on
Windows, and XDG portal/Zenity fallback on Linux. Verify the macOS bundle's
folder-access purpose keys from `assets/macos-permissions.plist`. Native Linux
checks must exercise a distribution-provided portal backend or Zenity, with
`libdbus-1-3` available; no GTK/WebView runtime is bundled. macOS access denial
must not be worked around by silently changing ACLs or requesting Full Disk
Access.

macOS public distribution requires Developer ID signing/notarization; Windows
publisher signing needs owner credentials. Keep credentials outside source and
model context.

The optional native account flow requires a compatible Cloud deployment with
Desktop account protocol1. Configure the real HTTPS origin and
the provider's fixed browser callback, then verify signed-out browser continuation,
allow/deny, native return, restart/account refresh and self sign-out on each
advertised OS. Synthetic local proof does not establish live OAuth/email or TLS.
Local must remain usable without an account or reachable Cloud.

## Native rendering proof

Test-only `ui-smoke` builds support eframe's screenshot capture and timed normal
close in an explicitly empty test directory. Set `PATCHFLEET_NATIVE_SMOKE=1` to
prevent provider probing. These controls are absent from production builds.
The UI tests also paint all tabs without a display and exercise action payloads.
The `decisions` scene creates real disposable source/ADR events with proposed,
accepted, rejected and superseded states and opens an exact decision read.
The `projects`, `folder-picker` and `folder-error` test scenes create an isolated
Git fixture. The error scene removes write permission only from that fixture;
restore its mode during test cleanup. `PATCHFLEET_SMOKE_MIN_WINDOW=1` checks the
800×560 layout, and `PATCHFLEET_SMOKE_LANGUAGE=en` selects English. Check actual
native dialog opening, neutral cancellation and normal close with the sheet
open. These scene controls are also absent from production.
For a modal, use `PATCHFLEET_SMOKE_SCREENSHOT_TO=/absolute/test-image.png` rather
than eframe's early two-pass screenshot: it waits for sizing frames, requests
the rendered image, saves it and closes through Host shutdown. Its PNG writer
is an optional test-only dependency already used by eframe, not a new production
runtime component.

The screenshot feature reads the OpenGL framebuffer; it alone does not prove
that macOS presented the shipped window. Also launch the production App through
LaunchServices with an explicit disposable `PATCHFLEET_DATA_DIR`, then inspect
an OS capture of only that test process's window. Close all older Patchfleet
instances first: the shared bundle identifier can route an Applications launch
to an already-running 0.3 WebView copy. Record the actual launched PID/bundle,
version and executable hash. Keep provider probes disabled or bound to failing
test executables, and terminate only the test instances.

When a developer already has an older Patchfleet DMG mounted, the packager's
create-dmg helper may collide with its volume name. Preserve that owner mount.
Package the App first, then use hdiutil with a distinct versioned volume name
and a temporary folder containing the App, Applications shortcut and LICENSE.
Verify the resulting compressed image before handing it off.

## Rollback

Close the app and restore a compatible executable and verified data backup.
Native 0.4 keeps the 0.3 Rust data layout; no migration is required between them.
Native 0.5 reads that existing state and adds V3 project/source facts only when
new owner memory commands or bound work occur. The 0.3/0.4 readers cannot read
V3 events; rollback restores the stopped, unchanged pre-upgrade backup.
Native 0.6 additionally reads/writes V4 local decision commands and facts.
Creating or transitioning an ADR requires a 0.6 reader thereafter; a 0.5
rollback uses the unchanged pre-upgrade data copy. Merely opening an existing
valid log does not fabricate an ADR or upload its contents.
Native 0.7 additionally reads/writes V5 shared-memory queue, attempt and receipt
facts. Back up the closed data directory before the first shared-memory action;
rollback to 0.6 after a V5 write uses that unchanged pre-upgrade copy. Verify the
separate Memory protocol1 service before enabling publication in a deployment.
The `shared-reader` and `shared-publisher` UI smoke scenes require a disposable
loopback memory service and fresh fixture pairing codes; the private two-Host
integration driver supplies these. They never use an installed app or owner data.
The historical Node 0.2 reader cannot read local V2 audit events: restore its
unchanged pre-migration data when rolling back to Node. Never run two writers.

See [installation](install.md), [ADR 0024](decisions/0024-native-rust-desktop-ui.md)
and [current evidence](state/native-rust-desktop.md).

Native 0.10 writes V6 run-bound memory context facts only when preparing a
project-bound agent launch. Readers before 0.10 reject those facts. Preserve the
stopped pre-upgrade backup for rollback. Verify MCP inventory plus actual context
preflight against an isolated Codex 0.155.1+ without a model turn. Check two
worktrees, terminal revocation, stale context, and no source/query/token export.

Native0.11 adds V7 extraction settings/jobs/provenance when the owner uses the
new commands. Preserve the stopped pre-upgrade backup before enabling or
queueing; older readers reject V7. The extraction-memory and extraction-decisions
smoke scenes use deterministic canonical provider results without any live model
turn. Verify consent/default-off, source edit/disable cancellation and truthful
manual retry in addition to the existing Local acceptance lifecycle. See
[draft evidence](state/automatic-drafts.md).
