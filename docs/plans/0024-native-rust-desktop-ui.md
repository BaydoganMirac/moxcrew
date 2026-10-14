# Task 0024 Native Rust desktop interface

Status: Implemented locally; platform release checks pending

Owner approval: 2026-10-14, explicitly replace WebView with a fully Rust interface.

## Outcome

A Rust-only desktop executable and local macOS installer preserve the current
project registration, work control, agent/team configuration, provider
observation and optional Cloud flows. No browser engine or Node process is
started by the installed app. Reuse the accepted Rust core and data directory.

## Ownership

- Coordinator: root/Cargo manifests and lock, desktop main/lifecycle/command
  bridge, integration, old Tauri retirement, docs and final verification.
- Interface builder: `crates/patchfleet-desktop/src/ui.rs` and its UI helpers/tests.
  Render existing operational flows and emit typed actions; do not mutate core,
  provider, Cloud, manifests or coordinator lifecycle files.
- Packaging builder: `Packager.toml`, native CI workflow and desktop icon assets.
  Package the Rust binary; do not publish, push, edit UI/core or owner data.
- Review: inspect the stable integrated implementation and desktop evidence.

## Shared UI contract

`Console::default()` creates form state. `Console::show(&mut self, &mut egui::Ui,
&serde_json::Value, busy: bool) -> Vec<crate::command::Command>` renders panels.
`Command::new(&'static str, Value)` attaches a durable five-minute owner envelope.
The snapshot retains the existing core shapes. The coordinator handles async
dispatch, safe error feedback, exact command retry and closing. Interface code
does not call provider programs or change projections. eframe is pinned to
0.36.2 with native rendering, fonts and accessibility enabled.

## Acceptance

- Native Rust UI renders and supports all existing action namespaces.
- Owner forms preserve target revisions and idempotency identities.
- A failed host open is visible in the window rather than a blank screen.
- Provider/Cloud operations do not block rendering; one writer still owns data.
- Normal close awaits owned process cleanup. No blind recovery side effects.
- Rust tests/format/Clippy pass and the actual macOS native frame is verified.
- Installer integrity and absence of WebView/Node runtime dependencies are
  checked; platform CI is configured without claiming unrun builds.
- No live data, provider settings or installed application are modified by tests.
