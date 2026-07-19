# ADR 0019: Use declarative packs and local team orchestration

Status: Accepted

Date: 2026-07-20

Supersedes: the product-order deferral in ADR 0017 and the public roadmap

## Context

The owner approved agent packs and multi-agent teams as the next product phase.
Patchfleet already has a local workspace registry, append-only events, durable
work intents and receipts, and bounded Codex start/cancel control. The safest
extension is to compose those proven parts without introducing executable
plugins or hosted agent execution.

## Decision

Agent packs are strict versioned JSON data. They declare identity, role,
instructions, supported provider, capability requirements, permissions, model
preference, work limits, expected output, tests, and provenance. Installation
copies validated data into Local storage; it never imports or evaluates code.

A team is a durable local aggregate containing an immutable configuration and a
validated task dependency graph. Exactly one orchestrator pack coordinates it.
The scheduler creates ordinary work intents for ready tasks, enforces bounded
concurrency and retry limits, and records decisions and receipts in the existing
single-writer event log. Codex is initially the only controllable provider;
other providers remain observable until their supported control surfaces meet
the same lifecycle contract.

Local remains the execution authority. Restart reconciliation never claims a
provider session can resume: active lost attempts become interrupted, pending
tasks remain, and the owner may explicitly retry.

## Consequences

- Ready roles and teams become useful without a plugin runtime or new service.
- Pack instructions, paths, credentials, transcripts, and tool output remain
  local-only.
- Historical teams retain manifest snapshots even if a custom pack is removed.
- Executable plugins still require two real needs, sandboxing, signing, update
  policy, revocation, and a separate threat-model ADR.

## References

- [Task 0019](../plans/0019-declarative-agent-packs.md)
- [Task 0020](../plans/0020-local-agent-teams.md)
- [ADR 0004](0004-append-only-local-events.md)
- [ADR 0014](0014-codex-control-uses-a-bounded-app-server-session.md)
