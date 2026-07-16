# ADR 0015: Phase 3 uses outbound polling and remote cancel

Status: Accepted

Date: 2026-07-16

## Context

Phase 2 made local observation, work state, Codex start/cancel, and terminal
receipts durable. Phase 3 must prove the optional Cloud boundary without
uploading local work text or paths and without making Cloud an execution
authority.

Remote enqueue is not yet safe: the local command requires an absolute Git
worktree path, while paths are forbidden in default Cloud payloads. A workspace
alias contract does not exist.

## Decision

Phase 3 is one single-user paired control loop:

1. Cloud creates a ten-minute, single-use pairing code.
2. The local host consumes it over outbound HTTPS. Plain HTTP is allowed only
   for exact loopback development origins.
3. Cloud returns one opaque host ID and one revocable credential. Cloud stores
   only its SHA-256 digest; the host stores the credential in its mode-0600
   local data file.
4. The host polls through its existing local Node application. No inbound host
   port, separate daemon, WebSocket, or provider credential transfer is added.
5. Heartbeats and projections are constructed field by field. The projection
   contains only opaque IDs, provider/version/capability metadata, lifecycle
   status, revisions, queue position, and coarse timestamps.
6. The only remote mutation is `cancel_run`. Cloud records an expiring,
   idempotent intent; the local host validates and applies it through the Phase
   2 command engine; only the local receipt terminalizes it.
7. Host revocation blocks heartbeat, projection, polling, and receipts on the
   next request.

The local launcher calls a token-protected loopback sync route on a bounded
poll interval so all local writes still pass through the existing Next.js
process and canonical writer.

## Consequences

- Cloud can show sanitized host/provider/work/run state and pending/terminal
  cancel outcomes.
- Cloud unavailability never blocks local operation; failed receipt delivery is
  retried using the original local semantic receipt.
- Remote enqueue, questions, queue editing, auth vendors, billing,
  notifications, packaging, and provider-control generalization remain
  deferred.
- A future remote-enqueue phase must first introduce a local workspace-alias
  contract that does not reveal filesystem paths.

## References

- [ADR 0003](0003-sanitized-projections-and-durable-intents.md)
- [Host-to-Cloud protocol](../protocol.md)
- [Phase 3 plan](../plans/v0-paired-cloud-control.md)
