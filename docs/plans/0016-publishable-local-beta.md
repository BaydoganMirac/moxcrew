# Task 0016: Publishable Local beta

Status: Approved

Owner approval: 2026-07-17

## Goal

Make the packaged Local product diagnosable and verifiable on macOS, Windows,
and Linux without changing the canonical event model or Cloud protocol.

## Scope

- Replace the end-of-life Node 20 package floor with ADR 0018.
- Add a dependency-free `patchfleet doctor` command for runtime, local state,
  workspace, and optional Cloud connection diagnostics.
- Add Linux and Node 24 compatibility coverage to CI.
- Extend the clean-package lifecycle smoke through diagnostics and recovery.
- Make install, upgrade, uninstall, backup, recovery, and compatibility docs
  task-oriented.

## Out of scope

- Native installers, OS login services, telemetry, new provider controls, new
  remote intents, package publication, deployment, or executable plugins.

## Acceptance criteria

1. `patchfleet doctor` reports bounded pass/warn/fail checks without printing
   credentials, prompts, provider output, or local event contents.
2. An empty first install is healthy with actionable warnings; corrupt durable
   state or an unsupported runtime fails with a non-zero status.
3. macOS, Windows, and Linux run the release path on Node 22, and Linux also
   verifies Node 24.
4. The packaged CLI passes install, doctor, workspace, start, status, stop, and
   recovery checks.
5. Existing local control, persistence, browser-boundary, and privacy tests
   remain green.

## Verification

- CLI diagnostics tests.
- Full public test suite and production build.
- Clean package lifecycle smoke.
- Diff, secret, and forbidden-path review.
