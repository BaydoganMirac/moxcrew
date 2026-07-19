# Task 0020: Local agent teams

Status: Approved

Owner approval: 2026-07-20

Depends on: Task 0019

## User problem

An owner needs to assemble a useful delivery team from ready agents and receive
one understandable outcome instead of manually coordinating disconnected runs.

## Product slice

- Provide Product feature, Bug fix, SaaS launch, Design and frontend, Security
  audit, and Release team templates.
- Let the owner choose a registered workspace, one orchestrator, agent packs,
  concurrency, dependencies, approval points, retry limit, time budget, and
  failure policy.
- Convert the goal into a validated task graph, enqueue bounded Codex work, and
  persist assignments, attempts, handoffs, questions, approvals, cancellation,
  review, receipts, and terminal team state in the canonical local event log.
- Recover after restart without claiming that a lost provider session resumed;
  preserve pending tasks and require an explicit retry for an interrupted task.

## Acceptance criteria

1. Invalid graphs, cycles, incompatible providers, unavailable capabilities,
   excessive concurrency, and unsafe budgets fail before launch.
2. Only ready tasks run, concurrency is enforced, every attempt has one owner,
   and retries are bounded and auditable.
3. Cancellation stops new scheduling and uses the existing typed run-cancel
   control for active Codex runs.
4. Approval and question gates cannot be bypassed by an agent result.
5. Restart recovery, partial failure, retry, cancellation, idempotency, and a
   complete template journey have deterministic tests.

## Rollout and rollback

Teams are opt-in and existing single-work behavior is unchanged. Rollback stops
scheduling, preserves append-only history, and leaves active provider control to
the existing safe reconciliation path.

## Deliberately excluded

Hosted model execution, model resale, provider credential storage, arbitrary
shell commands, unbounded autonomous planning, and unsupported provider control.
