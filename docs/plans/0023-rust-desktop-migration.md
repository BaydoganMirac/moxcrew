# Task 0023 Rust desktop migration

Status: Implemented locally; native release checks pending

Owner direction: 2026-10-14 — migrate the local platform to Rust and make it a
desktop application for macOS, Windows and Linux.

## Objective

A Tauri desktop application runs the local Patchfleet host in Rust, without
spawning the old Node/Next server. Preserve provider observation, supported
Codex control, durable work and workspace state, declarative packs and teams,
and the existing sanitized Cloud protocol. Cloud retains its separate web stack.
Project memory follows this migration; an extractor or vector index is not a
prerequisite for preserving the current product.

## Implementation ownership

- Coordinator: root Cargo workspace, host commands, work control, team
  scheduling, CLI, integration, governance/docs and release CI.
- Desktop builder: `desktop/`, `src-tauri/`, root package/lock and frontend
  configuration. Preserve user flows through Rust IPC, no Node sidecar.
- Integration builder: `crates/patchfleet-core/src/providers.rs` and `cloud.rs`,
  their tests and fixtures. Port supported surfaces, never scrape transcripts.
- Persistence builder: `crates/patchfleet-core/src/domain.rs` and `store.rs`,
  persistence/parity fixtures and tests. Preserve existing V1 event meaning.
- Independent review: after a stable integrated diff, inspect durability,
  cancellation, permissions, Cloud redaction and platform build configuration.

Workers do not publish, push, change Cloud infrastructure, or modify live user
data. File ownership stays disjoint. The owner request authorizes this bounded
migration; it supersedes the earlier Node-only deployment restriction.

## Shared Rust boundary

`patchfleet-core` exposes an owning Host. Desktop IPC offers `host_snapshot`
and `host_action` with allowlisted action and validated payload. Snapshot keeps
existing projection shapes: `observation`, `work`, `workspaces`, `packs`,
`templates`, `teams`, public `cloud` status, app version and current host epoch.
No arbitrary executable, shell string, path read, or credential read tool is
exposed. The core owns one locked append-only event log and background tick.

## Acceptance

- Rust persistence tests cover existing event replay, malformed completed
  lines, crash tail, duplicate identities, atomic projection recovery and a
  second-writer lock.
- Rust provider checks cover bounded probing, app-server handshake, completion,
  cancellation and session loss with fake processes; capabilities stay honest.
- Rust Cloud checks cover strict public schemas, allowlist redaction,
  same-outcome retries, expiry, revocation and no default private-data upload.
- Work, workspace and team commands survive restart with attributable durable
  receipts and bounded concurrency. Restart never claims ownership of an old
  provider process.
- Frontend typecheck/build and Rust test/check pass; build the native macOS
  application locally. CI builds/tests on macOS, Windows and Linux with native
  dependencies and produces desktop artifacts without publishing them.
- Desktop navigation cannot grant remote pages IPC access. CSP, restricted
  capabilities and single-instance ownership are explicit.
- Data migration uses copied fixtures or an explicit owner-selected directory;
  do not silently rewrite the existing installation during development.

## Delivery limits

Cross-platform build configuration is not evidence that unrun platform builds
passed. Signing and notarization require owner release credentials; local
development must not claim a signed public release. Record actual build/test
evidence and remaining platform checks in `docs/state/rust-desktop.md`.

## Inputs

- ADRs 0004, 0008, 0014, 0019, 0020, 0021 and 0022.
- Existing `lib/domain/`, `lib/providers/`, `lib/cloud/` and behavior tests are
  migration references, not runtime dependencies of the desktop application.
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri IPC](https://v2.tauri.app/develop/calling-rust/)
