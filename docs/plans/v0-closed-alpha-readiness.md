# Phase 4: Closed Alpha Readiness

Status: Complete

Owner: Patchfleet coordinator

Approved by owner: 2026-07-17

Updated: 2026-07-17

## Outcome

An invited owner can install the public Patchfleet tarball on a clean Node 20+
macOS or Windows environment, start and recover the local host without a source
checkout, authenticate to a deploy-ready private Cloud, pair, observe, cancel,
receipt, revoke, and keep working through Cloud failure.

## Delivery sequence

1. [Task 0012](0012-public-installable-host.md) packages the existing local
   deployable behind a minimal cross-platform CLI.
2. Verify pairing and host recovery against the public V1 protocol.
3. Package, database, production-build, and cross-repository HTTP smokes prove
   the release candidate before closure.

## Locked scope

- npm tarball and `patchfleet start|stop|status|recover`;
- private per-user data, runtime metadata, and log paths;
- stale-process detection and deterministic projection recovery;
- no public protocol change.

## Done gate

- `npm pack` contains the production app, CLI, Gemini extension, and no tests,
  local state, secrets, or repository metadata.
- A clean prefix install can start, report status, stop, and recover from the
  durable event log without losing canonical state.
- The signed-in owner can access only their own workspace.
- Host authorization, redaction, replay, expiry, revision, revocation, and
  receipt tests remain green without widening V1.
- Public tests, the production build, package smoke and paired HTTP checks pass.
- No unresolved P0-P2 review finding remains.
- Commits stay local; no package publish, migration apply to a hosted database,
  production resource creation, push, or deployment occurs.

## Deferred

- remote question answering, enqueue, queue editing, and additional providers;
- billing, notifications, retention history, teams, and entitlements;
- OS login startup, signed installers, Electron/Tauri, and auto-update;
- provisioning any hosted service.

## Completion evidence

- Public tests passed 130/130 and the final production build passed.
- A packed clean global-prefix install used npm's real executable shim and
  passed `start`, `status`, `stop`, and deterministic `recover`.
- The package contains no repository metadata, test suite, environment file,
  secret, or machine-specific absolute build path.
- Cloud unit/contract tests and production build passed with zero audited npm
  vulnerabilities.
- The real cross-repository HTTP smoke passed pair, sanitized projection,
  cancel receipt, revocation, authorization denial, and Cloud outage.
- No package was published, hosted migration applied, resource created, branch
  pushed, or deployment performed.
