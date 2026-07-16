# Task 0012: Public installable host

Status: Complete

Coordinator: Patchfleet coordinator

Approved by owner: 2026-07-17

## Objective

Turn the existing Node/Next.js local deployable into a publish-ready npm
tarball without changing its execution, provider, event, command, or Cloud
contracts.

## Owned files

- `bin/patchfleet.mjs` and focused CLI/package tests;
- `scripts/local-next.mjs` and one local runtime-health route;
- package metadata and build inclusion rules;
- Gemini extension data-directory resolution;
- install, upgrade, recovery, plan, decision, and state documentation.

## Acceptance

- Node 20+ and npm native `bin` shims cover macOS and Windows;
- start is idempotent and waits for the exact runtime identity;
- status and stop reject stale PID/port metadata instead of touching an
  unrelated process;
- state, connection credentials, PID metadata, and logs are owner-only;
- recover rebuilds derived projections only from the durable local event log;
- a packed clean install passes lifecycle smoke;
- local tests and production build remain green;
- no dependency, desktop shell, service installer, publish, or push.

## Forbidden scope

- moving provider control or canonical state into the CLI;
- automatic OS login startup, privilege escalation, or system-wide service;
- package registry publication or release signing;
- protocol or Cloud product feature changes.

## Completion

The dependency-free CLI, relocatable sanitized production artifact, macOS and
Windows CI matrix, install/upgrade/recovery contract, and real npm-shim package
smoke are complete. The package remains unpublished for owner review.
