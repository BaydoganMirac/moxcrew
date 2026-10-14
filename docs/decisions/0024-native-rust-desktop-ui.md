# ADR 0024 Native Rust desktop interface

Status: Accepted

Date: 2026-10-14

Owner direction: rewrite the interface in Rust and remove WebView entirely.
Supersedes the Tauri/React interface and IPC clauses of ADR 0023. Its Rust host,
canonical data, provider control and optional Cloud boundary remain accepted.

## Decision

Use an egui/eframe native desktop window. The interface and lifecycle are Rust;
the production executable embeds no browser engine, HTML or JavaScript assets.
Reuse patchfleet-core directly. A bounded async bridge submits typed owner
actions without blocking rendering, keeps retry identities and awaits host
shutdown on normal exit. Preserve the existing application data directory.

Native controls here mean a native window with Rust-rendered egui widgets;
they are not AppKit/WinUI widgets. Accessibility support is enabled. Idle
rendering is event driven with bounded status refreshes. The UI is not a web
application and is not compiled to WebAssembly.

The owner prioritizes responsiveness, low RAM and lightweight distribution.
Use one Glow renderer rather than bundling multiple graphics backends. Avoid
continuous redraw, web-engine processes and frequent idle CLI probes. Measure
actual native startup, idle CPU and memory; do not infer performance from the
implementation language alone.

Package the standalone Rust binary with cargo-packager. Native CI builds target
macOS ARM64/Intel, Windows x64 and Linux x64/ARM64. Signing and publication remain
separate owner release actions. A native macOS startup/render proof is required
before handing off its installer; unrun platforms are recorded honestly.

The old Tauri 0.3 startup defect rejected its empty-path bundled origin. It was
reproduced with a regression test and a native WebView probe. The owner's new
direction replaces that interface rather than shipping another WebView build.

## References

- [eframe](https://docs.rs/eframe/0.36.2/eframe/)
- [Packager](https://github.com/crabnebula-dev/cargo-packager)
- [Native interface task](../plans/0024-native-rust-desktop-ui.md)
