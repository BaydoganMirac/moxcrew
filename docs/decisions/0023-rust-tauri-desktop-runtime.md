# ADR 0023 Rust and Tauri desktop runtime

Status: Accepted

Date: 2026-10-14

Owner direction: migrate the local platform to Rust and deliver a desktop app
for macOS, Windows and Linux.

Supersedes the deployment and distribution assumptions in ADRs 0005, 0016,
0018 and the Node service packaging clause of ADR 0021. Refines the proposed
Rust direction in ADR 0022; the owner now explicitly requests a desktop shell.

## Decision

The local product uses `patchfleet-core` for canonical events, projections,
workspace and work commands, provider processes, team scheduling and outbound
Cloud sync. Tauri embeds this Rust host with a static React/Vite interface.
Node is used to develop/build the interface and run legacy reference tests;
the installed application does not launch a Node/Next server.

The desktop exposes only validated `host_snapshot` and `host_action` IPC to its
bundled main window. Remote navigation and new windows are denied. One OS writer
lock owns the data directory, and an in-process mutex serializes mutation.
Legacy Node writes refuse directories containing the Rust writer marker.

Preserve existing V1 event and public Cloud protocol meaning. New local V2
owner and remote-team command audit records make request/receipt retries
durable. Vector search remains a future derived index; no database replaces
canonical acceptance facts in this migration.

Provider permissions and supported capabilities remain unchanged. Codex starts
use two durable checkpoints before the turn side effect. Restart never
reattaches a lost process or silently repeats uncertain starts. Background team
advancement runs without a browser refresh. Closing the desktop stops owned
provider processes; opening another instance focuses the owning window.

Patchfleet Cloud retains its independent web stack and existing V1/V2 protocol.
A damaged optional Cloud connection file disables that integration while local
work remains available. It is preserved for explicit owner recovery.

## Distribution and migration

Build platform installers on native CI runners. macOS supports ARM64 and Intel
targets; Windows uses MSVC and WebView2; Linux uses system WebKitGTK with x64 and
ARM64 build entries. Signing/notarization and publication remain owner release
actions. A build configuration is not proof that an unrun target passed.

The desktop uses its platform application-data directory. Do not automatically
rewrite the legacy `~/.patchfleet` installation. Import an offline copy into a
new empty destination and validate it before using it. V2 local audit events
are not readable by the old Node host; retain the pre-cutover backup for rollback.

## Consequences

- The same Rust core serves the desktop and optional headless/offline CLI.
- Canonical events remain append-only and projections rebuildable. The owning
  host caches accepted state; unchanged UI reads do not replay or fsync history.
- Private files use Unix modes or protected owner+SYSTEM Windows ACLs, and
  symlinks/reparse points cannot redirect private state file operations.
- Display-name ordering uses deterministic Unicode order across platforms.
- Provider CLIs and their own installation requirements remain separate.
- Project identity, message ingestion, scoped MCP and automatic ADR drafts
  continue under Task 0022 using the Rust host; they are not claimed as shipped.

## Evidence

See [Task 0023](../plans/0023-rust-desktop-migration.md) and
[runtime state](../state/rust-desktop.md) for actual checks and platform limits.
