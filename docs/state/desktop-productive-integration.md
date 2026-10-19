# Productive crew desktop integration

Updated: 2026-10-19

Status: Implemented; integrated native capture and independent review in progress.

Plan: [Task0044](../plans/0044-productive-project-crews.md).
Decision: [ADR0045](../decisions/0045-productive-crews-and-explicit-tools.md).

## Native surfaces

Chat retains its goal composer and explicit prepare/start lifecycle. Mox planner
is an explicit provider operation with quota disclosure, missing questions,
scoped answer replanning, editable roster and member provider overrides. Bundled
guide planning remains available without inference. Completed plans and missing
questions replace the expanded request form so the next action fits the minimum
window. Saved plan inspection does not overwrite an unsent owner goal.

The existing crew surface opens project-bound direction/provider controls,
recipes, schedules and activity. Provider assignments and job/time/review limits
are saved explicitly. Direction names actual affected task IDs and observes the
Host's asynchronous state. Recipes prepare only; schedules require an explicit
create/enable action and disclose host-awake execution. Handoffs, reviewer
reported checks, real starts/elapsed time and nullable usage measurements are
read from scoped records. Missing token/cost measurements remain unknown.

Output Studio opens from Chat, without another primary navigation route. Its
artifact/version picker binds previews, selected-section revisions and export to
one Project, work ID, path and SHA-256. A revision prepares a new task; originals
remain intact. Office pages are text outlines, media is metadata/thumbnail, and
unsupported formats keep export available. PNG headers/dimensions are bounded
before native decoding.

Outputs, Resources, Tools and Apps use four internal tabs so connection setup is
reachable without traversing tool cards. Resources use a frozen native file-picker request. Tool executable selection is
explicit and labelled host-global, including optional PDF text support. App
credentials use a private password widget with no retained undo history. Drive
import, Notion page and Calendar event operations show an exact prepared request
before explicit execution. A changed form invalidates its preview. An uncertain
remote write is visible and has no automatic retry.

## Scope and retention

Bridge pages match the original command, Project and target identity; newer
requests reject older responses. Project switches/removal and Host epoch changes
clear productive overlays. Unsent owner text remains separate from transient
plan, file and credential scopes. Async preparation clears only the exact frozen
goal, roster and prior-file selection acknowledged by a real mission.

Connection tokens never enter Bridge pending/debug copies or failed-command
retry state. They clear after submission, close, navigation, Project change or
removal. Remote execute failures also do not expose an automatic retry action.
The Host remains the only writer and validates every native command.

## Validation

The escalated all-feature desktop suite passed 146 tests, including the existing
loopback account fixtures. After the final planner reveal adjustment, all 42
focused Create/crew/output tests passed, including actual pointer and keyboard
actions, TR/EN, both themes and 800x560 containment. The added minimum planner test
checks visible and clickable Prepare/Send answers without starting a provider.
All six Output Studio tests also pass after the Tools/Apps split, including
visible and clickable prepared-app approval at 800x560 in both languages/themes
and no resubmission after an unknown remote result. A separate Bridge regression
covers exact response identity, epoch invalidation and credential redaction. All-feature desktop strict Clippy passed after the final planner reveal; the
coordinator runs final workspace checks.

`ui-smoke` scenes `productive-plan`, `productive-questions`, `productive-direction`,
`productive-recipes`, `productive-schedules`, `productive-activity`,
`productive-output`, `productive-resources`, `productive-tools`,
`productive-apps` and `productive-app-preview` support native
QA. Actual disposable files/resources feed output scenes. Model result scenes
are isolated rendering fixtures and make no provider-execution claim. No live
provider inference, login, external account mutation or software installation was
performed by this UI implementation.

## Review corrections

Model-plan acceptance now includes valid owner-edited crew name and concurrency,
retry and time settings instead of silently restoring the generated values.
Schedule dates accepted by the native RFC 3339 field are normalized to UTC with
canonical milliseconds before dispatch, including offset and submillisecond
input. Saved plans expose the same explicit bounded paging as recipes/schedules.
All 14 focused crew tests pass, including actual Prepare/Schedule click payloads
and keyboard-driven plan pagination with exact Project/section/offset.

Final native polish uses friendly app names and readable localized label/value
rows for every frozen Drive/Notion/Calendar argument. Owner values remain exact;
the prepared action identity and explicit approval are unchanged. Usage waiting
reasons now read the actual `{taskId, reasonCode}` bottleneck shape. The final
all-feature desktop suite passes 151/151, including seven Output Studio tests,
minimum-window approval controls and actual-object bottleneck rendering.
