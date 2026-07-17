# V0 local work control

Status: Done

Last updated: 2026-07-17

## Done

- Owner authorized completion of Phase 2.
- Inspected the completed Phase 1 runtime and the earlier RiderHub work-intake
  and Codex app-server evidence.
- Verified Codex 0.144.1's generated machine-readable protocol includes the
  required start, resume, read, and interrupt methods.
- Accepted ADR 0013 for a shared canonical log with a separate local work
  projection.
- Accepted ADR 0014 for bounded Codex app-server control.
- Approved Task cards 0009 and 0010.
- Implemented Task 0009 exact version-one WorkItem, local command intent, and
  terminal receipt contracts.
- Added `enqueue_work` and revision-aware `remove_queued_work` through the one
  serialized, fsynced `events.jsonl` writer.
- Added the atomic, rebuildable local-only `work-items.json` projection with
  oldest-first work items, monotonic revisions, and safe receipt history.
- Covered restart replay, pending-request recovery, crash-tail repair,
  corruption failure, stale and expired commands, concurrent duplicate
  delivery, idempotency conflict, shared-log isolation, and privacy canaries.
- Closed Task 0009 review findings: pending requests expire without a work
  fact, work-item identities cannot be reused after removal, receipt reasons
  are bound to command replay semantics, and missing, stale, or corrupt work
  projections self-heal from the valid canonical log.
- Expiry is evaluated only after a command reaches the serialized writer;
  queued commands cannot apply with a stale pre-queue clock sample, while
  terminal duplicates still return the original receipt unchanged.
- Task 0009 independently re-reviewed clean with no unresolved P0-P3 finding.
- Task 0009 exact-tree checks passed: 93 tests, production build, and diff
  check; commits `3f7f357`, `ad66a4c`, and `649f001` remain local only.
- Implemented Task 0010 `start_work` and `cancel_run` with durable requested and
  terminal receipt events, opaque Codex run links, restart replay, stale and
  expiry validation, and fail-closed uncertain outcomes.
- Added bounded Codex workspace preflight for a real Git worktree root and a
  single long-lived app-server connection with `workspace-write`, approval
  policy `never`, persistent threads, raw events disabled, rejected server
  requests, and discarded notifications/native output.
- Added an fsynced owner-epoch launch contract before every provider call.
  `run.launching`, `run.prepared`, `turn.requested`, and `run.started` carry the
  same opaque boot owner, while the event log stores no provider-native item.
- Bounded real Codex 0.144.1 diagnostics proved that a new app-server rejects
  resume for both an empty prepared thread and an active turn after the owning
  app-server closes. Patchfleet therefore never attempts cross-process resume:
  old pending launches become `START_OUTCOME_UNKNOWN`/blocked and old active
  runs become session-lost/failed/blocked without a replacement start.
- Added the dependency-free loopback Next.js launcher that supplies one random
  owner epoch to all server workers for the lifetime of that local app boot.
- Added one exact same-origin, loopback-only, size-bounded URL-encoded mutation
  route with duplicate and extra field rejection.
- Added the responsive, keyboard-usable local work console for enqueue, remove,
  capability-aware start/cancel, durable run state, and safe receipt history.
- Focused fail-closed control/runtime tests and local-shell restart tests pass,
  including crashes before provider prepare, after provider prepare, after
  turn request, uncertain turn start, old-owner active run, stale read-only GET,
  manual Refresh reconciliation, and zero replacement starts.
- Disposable real Codex smoke applied same-owner start and cancel and excluded
  the private canary from receipts. Resume-limit diagnostics and all temporary
  files/processes were cleaned up.
- Full Task 0010 Builder checks pass on the exact tree: 102 tests, production
  build, and diff check; the dependency lockfile is unchanged.
- Closed the first independent review findings: uncertain thread/turn starts
  and interrupts now terminalize in the same call; old-owner pending cancels
  receive a durable receipt; UI and POST share the stable Codex 0.144.1+
  compatibility gate.
- Closed the second review's concurrency finding: a simultaneous terminal
  observation and successful cancel now produces one idempotent fact and
  receipt without a second revision. Completed/failed races do not claim a
  cancellation succeeded.
- Exact retry repairs missing receipts after durable `run.started`,
  `run.start_unknown`, `run.interrupted`, or `run.session_lost` facts, including
  an incomplete receipt tail, without another provider call.
- Final independent re-review reports no unresolved P0-P2 finding. The final
  exact tree passes 120 tests, a production build, and `git diff --check`.
- Final disposable Codex 0.144.1 smoke produced `WORK_STARTED` then
  `RUN_CANCELLED`; item and run ended `interrupted`, privacy canaries remained
  absent, and temporary processes/directories were removed.
- Phase 2 implementation commits are local only; nothing was pushed or
  deployed.
- Task 0010 implementation and hardening are recorded in local commits
  `df4a725`, `10064c8`, and `b82db83`.
- Closed the first owner-operated local work form failure: incidental outer
  whitespace is normalized at the browser boundary, and relative or `~` Git
  worktree paths now return the specific safe
  `WORKSPACE_PATH_NOT_ABSOLUTE` guidance instead of `INVALID_COMMAND`.
- Clarified that the user first queues an item and then starts it from Work
  items; the strict domain, Git preflight, event, receipt, provider, and Cloud
  contracts are unchanged.
- Task 0013 checks pass: focused local shell 2/2, full public suite 130/130,
  production build, clean package lifecycle smoke, and live local host restart.
- Completed Task 0014 product-ready local activation without changing the work
  domain, provider boundary, dependencies, or Cloud protocol.
- The local console now leads with privacy, Local/Codex/Cloud readiness, a
  three-step first-run path, accessible allowlisted command feedback, stronger
  responsive hierarchy, and plain-language durable outcomes.
- A real owner flow on the live paired host produced `WORK_ENQUEUED`,
  `WORK_STARTED`, and `RUN_CANCELLED`; the item and run ended interrupted and
  the sanitized Cloud projection resumed successful sync.
- Task 0014 exact-tree checks pass: 130/130 tests, production build, clean
  package lifecycle smoke, desktop and 390-pixel browser review, and no
  horizontal overflow.
- Accepted ADR 0017 and completed Task 0015 for a local-only workspace
  registry. Add/list/remove use the canonical event writer, expiring
  idempotent commands, immutable receipts, and replayable projection state.
- The queue form now selects an opaque registered project ID that is resolved
  locally; missing, conflicting, unknown, and manual-relative inputs fail with
  safe allowlisted feedback. Absolute paths remain outside Cloud.
- Clean package smoke exercises workspace add/list/remove, and a real
  registered-project flow produced `WORK_ENQUEUED`, `WORK_STARTED`, and
  `RUN_CANCELLED` with the item/run interrupted and sanitized Cloud sync
  continuing.
- Task 0015 exact-tree checks pass: 134/134 tests, production build, clean
  package lifecycle smoke, and diff checks.

## In progress

- None. Phase 2 is closed.

## Next up

1. Complete the authenticated live Cloud-authored cancel and definitive
   receipt proof.
2. Run clean-install activation sessions with the first external alpha users.

## Blockers

- None.

## Open questions

- None in the approved Phase 2 scope.

## Decisions accepted

- Local work text and working directory remain local-only data.
- Registered workspace IDs and paths remain local-only; the browser sends an
  opaque local ID and the server resolves it before enqueue.
- Work facts and receipts reuse the one canonical event log.
- Codex is the sole Phase 2 execution provider.
- Codex work is workspace-write with no approval escalation.
- Codex control is same-owner-boot only; restart is at-most-once and fail-closed.
- Cloud and generalized provider control remain deferred.
- Declarative agent packs wait for external activation evidence; executable
  plugins and multi-agent orchestration require separate gates and ADRs.
