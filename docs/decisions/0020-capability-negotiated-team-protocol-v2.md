# ADR 0020: Add a capability-negotiated team protocol V2

Status: Accepted

Date: 2026-07-20

## Context

V1 proves sanitized Local-to-Cloud visibility and one typed remote cancel. Team
supervision needs additional state and actions but must not turn Cloud into a
remote shell or move provider authority out of Local.

## Decision

V2 is negotiated per paired host and coexists with V1. It adds sanitized team,
agent, task, decision, question, and receipt projections. It adds only these
typed remote intents: `cancel_team`, `cancel_agent`, `approve_decision`,
`reject_decision`, `answer_question`, and `start_team`.

`start_team` contains an opaque local workspace alias, installed pack IDs, a
bounded owner-authored goal, template ID, concurrency, retry, and time limits.
Every intent is short-lived and idempotent. Local validates compatibility,
current revision, workspace mapping, pack installation, budgets, and policy and
may reject any command. Cloud considers an action complete only after the Local
terminal receipt is synced.

Source, diffs, absolute paths, pack instructions, provider prompts, transcripts,
tool calls/output, environment data, secrets, and credentials are forbidden in
both directions. Cloud stores only rebuildable sanitized projections and action
receipts.

## Consequences

- Old hosts keep V1 behavior and never see unsupported controls.
- Remote supervision gains useful decisions without generic execution power.
- Protocol validators and paired tests must reject both unknown fields and
  forbidden content before V2 can be enabled.

## References

- [Task 0021](../plans/0021-cloud-team-supervision.md)
- [ADR 0003](0003-sanitized-projections-and-durable-intents.md)
- [ADR 0015](0015-phase-3-uses-outbound-polling-and-remote-cancel.md)
