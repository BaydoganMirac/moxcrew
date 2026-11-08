# Task 0103: Adaptive mission runtime

Status: Local adaptive control loop implemented; Cloud and live usefulness gates remain separate

Updated: 2026-11-08

Authorization: the owner asked for Mox to act on each user's specific problem
instead of fulfilling fixed workflows, then explicitly requested a roadmap and
implementation. This card authorizes repository code, tests and documentation.
It does not authorize a live provider call, external account action, deployment,
release, push or deletion.

## Problem

The current planner selects an ability and roster before the system has a durable
representation of the problem. Crew tasks are then derived from a coarse role
order, and a mission revision can retry instructions but cannot structurally
change its graph. Existing review, budget, scoped communication, typed tools and
event recovery are strong building blocks, but the control loop is still closer
to a selected workflow than request-specific reasoning.

## Target control loop

`request → frame → provisional plan → execute → observe → evaluate → replan`

The local runtime owns validation and authority at every arrow. A provider may
propose the next move, but cannot grant itself access, increase a budget, start a
prepared mission, dispatch an external write or declare unsupported evidence.

## Roadmap

### Phase 0 — Durable problem framing (delivered)

- Add a bounded problem frame to newly generated provider plans: summary,
  desired outcomes, constraints, unknowns and success criteria.
- Keep the frame in the existing durable planner event/read path and show it
  before the proposed crew.
- Preserve replay of older V11 plans and the exact shape of recipes and legacy
  bundled-guide plans.

### Phase 1 — Provisional mission graph (delivered)

- Replace roster-first output with explicit objectives, hypotheses, tasks,
  dependencies, checkpoints, stop conditions and per-branch budgets.
- Derive crew membership from required capabilities instead of treating roles
  as the plan.
- Retain an editable owner review and explicit Prepare/Start boundary.

### Phase 2 — Evented plan revisions (delivered)

- Add versioned append-only facts for task add, retire, replace, reorder and
  dependency change.
- Resume the exact accepted plan revision after restart and keep every prior
  revision auditable.
- Reject stale, cross-Project, over-budget and permission-widening mutations.

### Phase 3 — Evaluate and replan loop (delivered)

- Evaluate task results, errors, contradictory evidence and owner corrections
  against checkpoints and success criteria.
- Replan only when a material trigger exists; cap cycles, elapsed time, jobs and
  review rounds.
- Finish with evidence, ask one blocking question or report a concrete blocker;
  never synthesize success from task completion alone.

### Phase 4 — Bounded adaptive work (delivered within the prepared authority)

- Let an authenticated mission agent propose bounded subtasks at runtime.
- Restrict every new task to an existing prepared agent and that member's
  already approved ability. A revision cannot add credentials, providers,
  tools, abilities or crew members.
- Permit agents to propose work while the runtime alone validates and accepts
  graph changes. Dynamic roster or permission expansion is intentionally not a
  local-runtime feature.

### Phase 5 — Product experience (first native surface delivered)

- Present “what Mox understood”, current strategy, real progress, why the plan
  changed, what decision is needed and evidence for completion in one Project
  conversation.
- Keep advanced graph/revision detail inspectable without turning the composer
  into a workflow editor.
- Preserve drafts, owner corrections and prepare/start clarity in TR/EN and
  future catalog-backed locales.

### Phase 6 — Optional Cloud supervision (separate future authorization)

- Define a separate sanitized projection for frame/plan status, checkpoints and
  approval requests.
- Keep prompts, raw evidence, source content, tool output, credentials and local
  paths out of Cloud by default.
- Add remote actions only as separately versioned allowlisted intents.

### Phase 7 — Evaluation and release gates (local fixtures delivered; live gate open)

- Build a cross-domain request suite that cannot be passed by choosing a fixed
  recipe: ambiguous goals, missing tools, contradictory evidence, user
  corrections and partial failures.
- Measure correct clarification, useful plan change, unsupported-success rate,
  restart fidelity, budget compliance and privacy canaries.
- Require live-provider and human usefulness evidence separately from fixture
  and schema correctness.

## Implemented local contract

- Newly generated provider plans require exact, bounded `problemFrame` and
  `missionPlan` values: strategy, objectives, hypotheses, topologically ordered
  tasks, expected evidence, checkpoints and stop conditions.
- Prepare derives the executable crew dependency graph from that plan while
  preserving explicit owner Prepare/Start boundaries.
- Authenticated agents can call `crew_checkpoint` with `satisfied`, `replan` or
  `blocked`. Only the local runtime can append the accepted V17 plan revision.
- Replanning can retire pending work and add at most four tasks per checkpoint,
  up to twelve mission tasks and two replan cycles, within the existing job and
  elapsed-time budgets.
- A revision cannot cross Projects, reuse another run, widen an ability, invent
  an agent, mutate completed history or depend on unknown/retired work.
- Adaptive completion requires `satisfied` with every declared success criterion
  passed. A missing checkpoint or `blocked` verdict fails honestly instead of
  converting task completion into unsupported success.
- Restart rebuilds the exact accepted task graph, plan revision and evaluation
  history from append-only events. Existing V11 plans remain replayable.
- Native plan and mission cards expose the problem frame, provisional/current
  strategy, objective/checkpoint counts, latest evaluation and plan revision.

## Limits

This implementation does not claim that a schema creates intelligence. Result
quality still depends on the selected provider and evidence. It does not add
dynamic credentials, a new provider, a new ability or an unreviewed crew member.
It changes no Cloud payload and performs no remote supervision. Live-provider
cross-domain usefulness and human acceptance remain release gates.

## Completion evidence

Strict schemas, the fake provider planning lifecycle, bounded replan/satisfied/
blocked paths, nonce idempotency, permission-widening rejection, exact restart
replay and native rendering are covered by local tests. The earlier
`PROVIDER_CONTROL_FAILED` report was a sandboxed loopback/test-peer issue, not a
runtime failure; the peer contract now includes `crew_checkpoint` and the
affected end-to-end tests pass in an authorized local-loopback run. No live
provider, Cloud payload, deployment, release, push or external account was used.
