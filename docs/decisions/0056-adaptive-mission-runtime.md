# ADR 0056: Problem-driven adaptive mission runtime

Status: Accepted by owner-approved Task0103

Date: 2026-11-08

## Context

The current Project planner turns an owner request into one primary ability and
an editable roster of at most six members. The runtime then derives a mostly
fixed research, production and review graph. This is inspectable and safe, but
it makes the selected workflow shape the work before Mox has represented the
owner's actual problem, desired outcome, constraints, unknowns and evidence of
success.

The product direction is broader: Mox should adapt its reasoning and actions to
each request, revise its strategy when evidence or execution changes the
situation and stop only when explicit success criteria have been evaluated.

## Decision

Adopt a problem-driven **Adaptive Mission Runtime** as the target orchestration
model:

1. frame the request as a bounded `ProblemFrame`;
2. produce a provisional objective/task/checkpoint graph from that frame;
3. let the local runtime validate abilities, tools, permissions, budgets and
   owner gates before any action;
4. observe attributable results and failures;
5. evaluate them against the frame and current plan;
6. append an inspectable plan revision when work must be added, replaced,
   retired or reordered;
7. complete only after the declared criteria have evidence or the remaining
   blocker is reported honestly.

Provider models propose framing, plans and revisions. Rust remains the authority
for schemas, Project scope, file access, tool readiness, budgets, idempotency,
prepare/start separation and external side effects. Model output cannot widen a
permission or convert a suggestion into execution authority.

Recipes and composed workflow graphs remain useful as optional planning hints,
examples and owner-saved traces. They are not the primary product abstraction
and do not constrain a request to a preselected path.

New provider plans contain a bounded `problemFrame` and `missionPlan`. The latter
declares strategy, objectives, hypotheses, task dependencies, expected evidence,
checkpoints and stop conditions. It is stored inside the existing durable V11
planner fact and becomes an adaptive mission only after explicit Prepare/Start.
Existing V11 plans without these fields remain replayable; recipes and legacy
bundled-guide planning retain their compatible path.

An accepted runtime evaluation is the V17 append-only
`project_workflow.checkpoint_evaluated` fact. It contains the authenticated run
binding, expected mission and plan revisions, exact evaluation, complete revised
task graph and runtime-generated IDs for added tasks. The projector independently
revalidates the revision. Per checkpoint the runtime accepts `satisfied`,
`replan` or `blocked`; it caps added/retired work, total tasks, replan cycles,
jobs and elapsed time. Added work must use an existing prepared agent and that
member's approved ability. Dynamic permissions and crew membership remain an
owner boundary, not model output.

## Consequences

- Mox can expose what it understood and what would count as success before
  proposing agents.
- Hidden assumptions and non-blocking unknowns become inspectable inputs to
  later evaluation and replanning.
- Structural replanning and evaluator checkpoints are durable local behavior;
  dynamic crew/permission expansion is deliberately rejected.
- Dynamic behavior remains bounded by deterministic local policy and owner
  approval instead of becoming an unbounded autonomous loop.
- Cloud may later receive only a separately defined sanitized operational
  projection; raw frames, prompts, evidence and local files remain Local by
  default.
