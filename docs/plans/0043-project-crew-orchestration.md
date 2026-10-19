# Task 0043: Project crew orchestration

Status: Approved, implementing

Updated: 2026-10-19

Owner approval: “Onaylı başla seriden yap tasarımı da ona göre yönlendir”,
following the accepted direction: Mox chooses a useful crew from a goal,
owners can adjust/start it, reusable teams belong to Projects, and agents and
teams exchange real work messages. The conversational interface remains.

## Outcome

A general-purpose Project goal produces an inspectable crew recommendation.
The owner can edit the roster, prepare the crew mission and explicitly start
supported local work. Real bounded agent runs share checked upstream outputs,
exchange attributable messages, and expose progress/files in the chat's crew
panel. Saved rosters can be reused for another goal in the same Project.
Teams become a primary navigation item. Existing single-job conversation and
legacy Git team execution remain usable.

## Local contract

- Add V10 local events for reusable Project crews, missions and messages; do
  not reinterpret V1/V2/V8/V9 facts or change the Cloud protocol.
- `project_crew_plan` is a bounded local read using bundled role/ability guides
  and explicit goal terms. Report its basis as bundled guides, not a fabricated
  model response. Return a clear missing-output error for ambiguous goals.
- `project_crew_prepare` accepts Project/revision, goal/title, primary ability,
  optional reusable crew, bounded editable members and optional selected prior
  files. It durably prepares a mission but never starts a provider.
- Separate start, cancel and retry actions carry exact Project/mission/revision
  scope and normal expiring idempotent owner envelopes. One Host remains the
  only writer. Preserve acknowledged outcomes through restart/crash prefixes.
- Reuse managed Project jobs, Codex control, decision scope and actual file
  safety checks. Use bounded dependencies/concurrency/time/retries. Recover
  interrupted work visibly, never invent resumed provider sessions.
- Snapshot `projectCrews` has revision, crews and missions. Crews contain
  crewId, projectId, name, members and createdAt. Missions contain missionId,
  crewId, projectId, title, instruction, abilityId, revision, status, tasks,
  settings and timestamps. Task entries identify agent, work item, dependencies,
  attempt and real status. Full messages are a scoped bounded read.
- Reuse crew composition while giving each mission its own goal, run/files and
  communication scope. No implicit cross-Project context sharing.

## Agent communication

Extend the existing authenticated run-scoped MCP transport with typed crew
context, message-read and message-send tools. The Host derives sender and
Project from the active run; agent arguments cannot impersonate identities or
select another Project. Same-Project team recipients are discoverable. Validate
membership, active lease/epoch, idempotency, body/page limits and delivery scope.
Messages are work context and never accepted decisions or permissions.

Downstream tasks receive checked terminal upstream outputs in their own inputs
directory, with provenance and size/count limits. A message is shown as delivered
only after durable append; no fabricated replies or thinking transcript. Live
agents read their own inbox. Closed-run messages must remain explicitly pending
unless an actual bounded reply run is scheduled; do not claim that writing an
inbox resumed a finished agent. Keep any reply work within the mission budget.

## Ownership

- Runtime builder: new core project_crews.rs and focused tests; crew planning,
  V10 validation/projection/recovery helpers, lifecycle/scheduler/messages.
  Coordinate exact public helper signatures with root before integration.
- UI builder: chat.rs, create.rs, new crew.rs, i18n.rs; roster/mission/chat panel
  and reusable team UX. Root owns ui.rs navigation and Teams dispatch.
- Coordinator: shared Host/domain/store/creation/conversation integration,
  authenticated MCP exposure, bridge/main fixtures, documentation and packaging.
- Reviewer: stable integrated commit, runtime recovery/scope/security and actual
  native usability evidence. Workers do not push/publish/deploy.

## Verification

Fake supported CLI processes must exercise actual scheduling and MCP delivery,
dependency outputs, cancellation, retries, restart and receipts. Test forged
sender/cross-Project/old run/malformed/duplicate requests and Cloud canaries.
Native TR/EN and both themes must fit normal and minimum windows, preserve
drafts and exact asynchronous mission/file selection, and show honest states.
Run relevant workspace tests, strict Clippy, feature-off build and independent
review. Record what is implemented and any provider/platform limits.

No dependency download, API key, provider login, live inference, Cloud mutation,
push or deployment is needed for implementation or automated verification.
