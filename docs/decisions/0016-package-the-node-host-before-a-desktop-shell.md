# ADR 0016: Package the Node host before a desktop shell

Status: Accepted

Date: 2026-07-17

## Context

Phase 3 works from a source checkout, but a closed-alpha user needs a stable
command, private user-data directory, background lifecycle, and recovery path.
The local product must remain one Node/Next.js deployable and work on macOS and
Windows without introducing a second UI runtime.

## Options considered

- Electron or Tauri: friendly packaging, but duplicates runtime and release
  complexity before the web-first product proves the need.
- OS-specific installers and services: native startup behavior, but two
  maintenance paths and signing work before closed-alpha evidence.
- npm package with a Node CLI: uses the existing deployable and npm's native
  cross-platform `bin` shims; requires Node 20+.

## Decision

Publish-ready artifacts use the npm package `patchfleet` with one dependency-
free CLI exposing `start`, `stop`, `status`, and `recover`. It launches the
existing production Next.js host in the background, stores runtime metadata,
logs, and canonical state under the user's Patchfleet data directory, and
never moves execution into the CLI.

## Consequences

- One tarball can be clean-installed and exercised on macOS and Windows.
- Source development keeps `npm run dev`; package publication and code signing
  remain owner-controlled release actions.
- OS login startup, Electron/Tauri, native menus, auto-update, and signed
  installers wait for real alpha demand.

## References

- [Architecture](../architecture.md)
- [Phase 4 plan](../plans/v0-closed-alpha-readiness.md)
- [npm package `bin` documentation](https://docs.npmjs.com/files/package.json/)
