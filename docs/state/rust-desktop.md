# Rust and Tauri desktop state

Status: Historical 0.3 migration; interface superseded by native Rust 0.4

Updated: 2026-10-14

The owner selected a WebView-free lightweight interface under [ADR 0024](../decisions/0024-native-rust-desktop-ui.md). Current implementation and artifacts are in [native state](native-rust-desktop.md). The evidence below records the earlier 0.3 migration.

## Delivered

The local 0.3 product runs in `patchfleet-core` inside the Tauri desktop shell.
Static React/Vite assets use restricted `host_snapshot` and `host_action` IPC.
The installed application starts no Node/Next server or Node sidecar. Node is
frontend build tooling and an explicit legacy behavior reference.

The port preserves provider observation, supported Codex start/cancel, work
intake and retries, workspace registration, twelve packs, six team templates,
owner gates, bounded team scheduling and optional sanitized Cloud V1/V2 sync.
Claude Code and Gemini CLI remain observation-only. Native Gemini hook ingress
stores lifecycle metadata in an inbox without taking canonical writer ownership.

One OS-locked Rust writer fsyncs canonical events. Derived state is cached in
memory and atomically persisted; reads do not replay or rewrite history.
Private files reject symlinks, reparse points and hard links. Windows uses
protected current-owner+SYSTEM ACLs; Unix uses private modes. The legacy reader
and writer refuse directories marked for the Rust host.

V1 replay preserves existing event meanings and command receipts. New V2
records audit local team/pack and remote team commands. Pending pure state
changes recover receipts from their applied facts. Old process ownership is
lost rather than reattached, and uncertain provider side effects are not resent.
Missing workspaces do not stop unrelated teams or Cloud sync. Drafts cannot
start after workspace removal; a new registration requires a new team.

Cloud data stays allowlisted. Pairing corruption reports recovery needed while
local work remains available. Credentials, local work text, absolute paths,
instructions and provider output never appear in the public Cloud status.

## Validation evidence

Validated on macOS ARM64 with Rust 1.99.0 and Node 24:

- Rust workspace tests: 42 passed; one manual timing probe excluded from the
  ordinary suite. This includes 19 persistence, eight host, eight provider,
  five Cloud and two desktop security tests.
- V1 parity: all five projections match the old implementation at 246 event
  prefixes across eleven real reference scenarios.
- A 52,000-event manual probe confirms cached reads below one millisecond with
  no projection writes. Startup still replays the canonical history.
- Frontend: four command tests, TypeScript checking and production build pass.
- Legacy event/protocol reference: all 149 tests pass. Its package check now
  verifies the desktop build and retirement of npm publication.
- Rust formatting and Clippy for the workspace/all targets pass with warnings
  treated as errors.
- UI smoke: nine mocked IPC flows and rendered layout were checked, including
  older WebKit form submission behavior. This is not native GUI runtime proof.
- Isolated Windows Store code typechecks and passes Clippy against the Windows
  standard library. It does not establish native Windows ACL behavior.
- Full Windows core cross-check was attempted on macOS. Its `ring` C build
  requires Windows SDK/UCRT headers (`assert.h`); this host lacks them, so the
  full target check remains for the native Windows runner.
- Independent review verified receipt crash recovery, duplicate removal
  receipts, missing-workspace starts, cached reads and instruction length parity.

The native CI matrix targets macOS ARM64/Intel, Windows x64 and Linux x64/ARM64.
It runs frontend/Rust checks and creates App/DMG, NSIS/MSI or Deb/AppImage
development artifacts without publishing a release.

Final macOS ARM64 release artifacts were built and verified locally:

- `target/release/bundle/macos/Patchfleet.app` — 9.51 MiB; ad hoc signature
  passes `codesign --verify --deep --strict`.
- `target/release/bundle/dmg/Patchfleet_0.3.0_aarch64.dmg` — 4,242,682 bytes;
  `hdiutil verify` passes.
- DMG SHA-256:
  `4c1d7bdebb6bf374f60150e9c05a039b02e5360773db2022a59198a37c3c525d`.
- The final bundled executable passes native hook ingress and invalid-argument
  smoke checks in a disposable directory. Only five sanitized signal fields
  are stored; no canonical writer or provider process starts.

Artifacts remain ignored build output. No release, installer upload, push,
Cloud deployment or live-data migration was performed.

## Remaining release evidence

- Run the native CI matrix and inspect each platform artifact. No Windows,
  Linux or Intel macOS native build/runtime success is claimed locally.
- Exercise the native desktop GUI, installation and normal exit on each
  advertised OS. Local browser fixtures cover UI behavior; this session did
  not automate the native macOS window.
- Run a consented live provider turn/cancel and staging Cloud pairing. Current
  conformance tests use fake native processes and loopback servers.
- Test offline-copy migration against an owner-selected installation. No live
  user data was migrated or rewritten during development.
- Configure public macOS signing/notarization and Windows publisher signing
  for a release. Development packages have no public signing credentials.

## Next product slice

Project-scoped local memory remains planned under ADR 0021. The Rust foundation
does not yet implement a stable Project model, source capture, vector index,
memory MCP tools or ADR extraction. Continue with source capture and canonical
decision lifecycle before adding the derived local vector index. Automatic
drafts become binding only after explicit owner acceptance.

## References

- [ADR 0023](../decisions/0023-rust-tauri-desktop-runtime.md)
- [Migration task](../plans/0023-rust-desktop-migration.md)
- [Installation and recovery](../install.md)
- [Desktop release](../release.md)
- [Project memory state](project-memory.md)
