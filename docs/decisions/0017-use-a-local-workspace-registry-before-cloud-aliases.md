# ADR 0017: Use a local workspace registry before Cloud aliases

Status: Accepted

Date: 2026-07-17

## Context

Local Codex control requires an absolute Git worktree root. Asking for that path
on every task is poor product ergonomics, while browsers cannot reliably grant
an arbitrary local directory path to a loopback web application. Absolute paths
are also forbidden from default Patchfleet Cloud payloads, and remote enqueue
has no workspace-alias contract yet.

## Options considered

- Browser directory upload: returns file handles or relative upload metadata,
  not a dependable absolute Git worktree root, and implies unintended file
  access.
- Native desktop picker: provides a friendly path selector but adds a second
  packaging runtime before closed-alpha demand proves it necessary.
- Cloud-owned workspace mapping: would move a local execution decision and
  sensitive path knowledge across the privacy boundary.
- Local registry plus CLI setup: works in the existing Node host, keeps paths
  local, and creates the prerequisite for a later opaque Cloud alias contract.

## Decision

Patchfleet stores registered workspaces as append-only local events and derives
a local projection. Registration canonicalizes and validates a Git worktree
through the same boundary used by Codex control. The CLI owns registry mutation;
the loopback UI selects an opaque workspace ID and the server resolves it to the
canonical local path.

The primary work form uses the registered-project selector. Manual absolute-path
entry remains under an advanced fallback for recovery and first use. If both or
neither are supplied, enqueue is rejected. The registry and its paths remain
local-only; the existing Cloud protocol is unchanged.

## Consequences

- Users register a repository once and select it by a stable local name on
  subsequent work.
- The Node package remains the only deployable and no filesystem dependency is
  added.
- A future remote-enqueue design can introduce a separate, explicitly approved
  opaque alias contract without reusing or uploading local paths.
- Native folder selection and plugin/agent installation remain later product
  stages, after the local control loop and integration contract are stable.

## References

- [ADR 0015](0015-phase-3-uses-outbound-polling-and-remote-cancel.md)
- [ADR 0016](0016-package-the-node-host-before-a-desktop-shell.md)
- [Task 0015](../plans/0015-local-workspace-registry.md)
