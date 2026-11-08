# Adaptive mission runtime

Updated: 2026-11-08. Task0103.

Status: Local adaptive control loop implemented; full local verification passed.

## Intended direction

Mox is moving from roster/workflow-first planning to a problem-driven control
loop. The accepted architecture is recorded in
[ADR0056](../decisions/0056-adaptive-mission-runtime.md), and the phased work is
bounded by [Task0103](../plans/0103-adaptive-mission-runtime.md).

## Delivered local runtime

- New provider plans require a bounded `problemFrame` plus a provisional
  `missionPlan` containing strategy, objectives, hypotheses, task dependencies,
  expected evidence, checkpoints and stop conditions.
- The prepared mission graph is derived from the plan rather than a generic role
  order. Owner edits and explicit Prepare/Start remain intact.
- The agent-memory surface now exposes `crew_checkpoint`. An authenticated
  mission agent may evaluate the exact declared criteria and propose a bounded
  task revision; only the Host accepts and events it.
- V17 `project_workflow.checkpoint_evaluated` facts remain append-only,
  revisioned, nonce-idempotent and replay-safe. New strict writes use V18 under
  ADR0057. Stale, cross-Project, over-budget, permission-widening and
  history-mutating revisions fail closed.
- Replan revisions can retire only pending work and add work only for an existing
  prepared agent using that member's approved ability. There is no implicit
  credential, provider, tool, ability or crew expansion.
- `satisfied` requires all declared criteria to pass after other work completes.
  `blocked` stops work and fails the mission with a durable evidence summary;
  an adaptive mission that omits its checkpoint fails instead of claiming success.
- Every checkpoint criterion now requires non-blank evidence. `satisfied` is the
  only verdict that may report every criterion as passed; `replan` and `blocked`
  must retain at least one explicit unmet criterion.
- A replan cannot retire a task while leaving pending or queued work dependent
  on it. Both command materialization and append-only replay validate the
  resulting schedulable dependency closure.
- New adaptive planner output is atomic: it must contain both `problemFrame` and
  `missionPlan`. Older V11 facts retain their accepted replay meaning.
- Task0104 lets an already prepared reviewer evaluate a terminal failed branch.
  Recovery allocation and checkpoint facts use V19/V20; failed outputs are not
  trusted inputs and legacy missions still fail fast.
- Native plan and mission cards show the problem, provisional/current strategy,
  objectives/checkpoints, latest verdict/summary and plan revision. New UI copy
  uses the central TR catalog and English fallback.
- Existing V11 planner facts without adaptive fields and recipe-derived plans
  remain compatible.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy -p patchfleet-core -p patchfleet-desktop --all-targets
  --all-features --locked --offline -- -D warnings`
- full serial core unit and integration suite; 140 core unit tests passed and
  environment/evidence-only tests remained explicitly ignored
- full native desktop suite; 293 tests passed
- focused core validation of legacy/new plan shapes and strict graph fields
- request-shape cases for blocking clarification and parallel analytics/support
  evidence followed by review
- blank evidence, contradictory verdict and orphaned replan-dependency rejection
- failed producer, prepared reviewer, V19 allocation and V20 recovery replan
- focused native planner question behavior
- full fake-peer planner/prepare/start behavior
- replan, satisfied and blocked checkpoint behavior, permission rejection,
  nonce replay and exact restart recovery
- focused desktop sync/render/acceptance behavior for frame and strategy state

The earlier `PROVIDER_CONTROL_FAILED` result was traced to local-loopback sandbox
denial and then to an outdated provider fixture tool list. The fixture now
advertises `crew_checkpoint`; its affected end-to-end paths pass when rerun
serially. No live provider, Cloud payload, external account or deployment was used.

## Remaining release gates

Live-provider cross-domain usefulness and human acceptance are not established by
fixtures. Optional sanitized Cloud supervision needs its own protocol/privacy
review and explicit implementation authorization. Dynamic roster or permission
expansion is intentionally excluded; unavailable authority must become an owner
question or an honest blocker.
