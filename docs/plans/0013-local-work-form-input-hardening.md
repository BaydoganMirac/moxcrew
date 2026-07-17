# Task 0013: Local work form input hardening

Status: Completed

Coordinator: Patchfleet coordinator

Approved by owner: 2026-07-17

## Problem

The first owner-operated local work attempt reached `/api/work` but returned
only `INVALID_COMMAND` before Codex control ran. The browser form does not
explain that the Git worktree must be an absolute path, and incidental outer
whitespace in copied title, instruction, or path values fails the strict domain
contract without a useful boundary error.

## Objective

Make the existing enqueue-then-start flow usable without changing the work
domain, Codex adapter, durable event model, or Cloud protocol.

## Owned files

- `app/api/work/route.ts`;
- `app/page.tsx`;
- focused assertions in `tests/local-shell.test.js`;
- `docs/state/v0-local-work-control.md`.

## Acceptance

- the route trims incidental outer whitespace before constructing a work item;
- a relative or `~` worktree value returns a safe specific error explaining
  that an absolute Git worktree path is required;
- the form tells the user to use the exact absolute worktree root and that
  enqueue precedes `Start Codex`;
- strict domain validation, Git worktree preflight, privacy rules, and all
  existing command semantics remain unchanged;
- focused route tests, the full suite, production build, and diff check pass;
- no dependency, lockfile, protocol, Cloud, or provider-control change.

## Completion evidence

- The browser boundary trims title, instruction, and worktree values while the
  strict domain validator remains unchanged.
- Relative and `~` paths return `WORKSPACE_PATH_NOT_ABSOLUTE` with a safe `pwd`
  instruction instead of the generic `INVALID_COMMAND` response.
- The local console explains the absolute-path and enqueue-before-start flow.
- The focused local-shell test passes 2/2, the full suite passes 130/130, the
  production build passes, and the clean package lifecycle smoke passes.
- The existing host restarted on the sanitized production build with its local
  Cloud connection and user data preserved.
