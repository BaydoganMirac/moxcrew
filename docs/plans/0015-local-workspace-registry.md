# Task 0015: Local workspace registry

Status: Completed

Owner approval: 2026-07-17

## Goal

Let a local owner register Git worktrees once and choose a project by name when
queueing work, without sending absolute paths to Patchfleet Cloud or requiring
a desktop shell.

## Scope

- Add append-only workspace registration and removal commands with durable
  receipts and a rebuildable local projection.
- Add dependency-free CLI commands to add, list, and remove registered
  workspaces.
- Resolve a selected opaque workspace ID to its canonical local path on the
  server before enqueueing work.
- Replace the primary raw path field with a registered-project selector while
  retaining manual absolute-path entry under an advanced fallback.
- Show registered project names in work items while keeping diagnostic paths
  local.

## Out of scope

- Cloud workspace aliases, remote enqueue, protocol fields, filesystem paths in
  Cloud payloads, or Cloud infrastructure changes.
- A browser directory upload control, desktop shell, native file picker, or OS
  service.
- Plugin installation, agent marketplace, dynamically loaded code, billing,
  telemetry, or package publication.

## Acceptance criteria

1. `patchfleet workspace add [path]`, `list`, and `remove <id>` operate through
   the canonical local event writer and produce terminal receipts.
2. Registration canonicalizes the path and accepts only an existing Git
   worktree root; filesystem root and the user's home directory are rejected.
3. Replaying the event log rebuilds an equivalent workspace projection, and
   duplicate or stale commands are safely rejected.
4. The work form accepts exactly one registered workspace selection or one
   manual absolute path. Workspace IDs are resolved locally and client-supplied
   paths cannot override a selection.
5. No workspace path or new workspace-registry field crosses the existing
   host-to-Cloud boundary.
6. Full tests, production build, clean package smoke, and a real local
   queue/start/cancel flow pass.

## Verification

- Workspace domain and persistence tests for replay, receipts, idempotency,
  corruption, canonicalization, and removal.
- CLI package tests for add/list/remove and help.
- Local-shell tests for empty, selected, manual, conflicting, and unknown
  workspace form states.
- Full public test suite, production build, package lifecycle smoke, and browser
  review at desktop and narrow mobile widths.

## Result

- Added a local-only workspace domain with registered/removed facts,
  idempotent expiring commands, immutable terminal receipts, and a rebuildable
  `workspaces.json` projection on the canonical writer.
- Added `patchfleet workspace add [path]`, `list`, and `remove <workspace-id>`;
  registration reuses the Codex Git worktree preflight and canonical path.
- Replaced primary path entry with a registered-project selector. The server
  resolves its opaque ID and rejects missing, conflicting, or unknown
  selection; one-off manual path entry remains under Advanced.
- Kept the Cloud protocol and sanitized projector unchanged. Workspace paths
  remain local and no executable plugin system was introduced.
- Passed 134/134 public tests, production build, clean tarball install and
  lifecycle smoke including workspace CLI commands, diff checks, and a real
  selected-project `WORK_ENQUEUED` -> `WORK_STARTED` -> `RUN_CANCELLED` flow.
