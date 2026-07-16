# Agent operating model

Status: working collaboration contract

Updated: 2026-07-16

## Recommendation now

Phase 1 observation, Phase 2 durable local control, and the owner-approved
Phase 3 paired Cloud control slice are complete. Collapse back to one
coordinator. Create another bounded team only for an approved packaging,
deployment-foundation, or provider-control milestone.

## Initial team

- Coordinator: owns product scope, shared contracts, task cards, integration,
  and final user handoff.
- Builder: owns one approved implementation slice with exact file boundaries.
- Reviewer: inspects the builder's stable commit for acceptance, durability,
  security, and scope.

The team bootstrap is complete when all three roles understand the same
AGENTS.md, applicable skills, file ownership, and stop conditions. Product code
starts only through an approved task card.

## When to add agents

### Phase 1A: one implementation owner

The builder owns the shared domain, local persistence, Codex adapter, and first
dashboard slice. The reviewer starts after the builder produces a stable local
commit.

### Phase 1B: bounded provider parallelism

After Codex passes the conformance harness:

- Claude adapter agent: provider folder plus adapter-specific tests only.
- Gemini adapter agent: provider folder plus adapter-specific tests only.
- Integration owner: shared contract and UI changes; resolves real differences
  rather than allowing adapters to widen the interface independently.

### Phase 2: runtime and UI split

If the files and tests now have stable ownership:

- Runtime agent: events, projections, queue, command application.
- Console agent: local screens and interaction states.
- QA agent: recovery, idempotency, cross-provider conformance, and security.

### Phase 3 and later: Cloud owner

With local control now useful, and only after the owner approves Phase 3:

- Cloud agent: private repository only.
- Protocol owner: public contract plus cross-repo contract tests.
- Security reviewer: pairing, authorization, redaction, revocation, and replay.

## Rules for every agent

- Receive one bounded task with owned files and explicit acceptance criteria.
- Read AGENTS.md, the active plan, relevant ADRs, and state before editing.
- Do not push, merge, deploy, publish, or migrate.
- Do not change shared schemas from a provider branch.
- Do not edit another active agent's files without coordination.
- Commit locally with a focused message and report tests plus remaining risks.
- Update state at handoff.
- Stop when repository behavior contradicts the product or protocol contract.

## Task card template

Each delegated task should specify:

- Objective: one observable outcome.
- Owned files: exact directories or files.
- Inputs: contracts and fixtures that must not be changed.
- Acceptance: commands and behavior that prove completion.
- Forbidden scope: adjacent work the agent must not absorb.
- Handoff: state file, local commit, tests, and open risks.

## Suggested next agent task

Do not infer the next milestone. The owner should choose between local
packaging/recovery, a production Cloud foundation, or another provider-control
proof. Each choice needs its own task card and must preserve the Phase 3 public
protocol and local-first boundary.
