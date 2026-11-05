# Productive Project workflows

Updated: 2026-11-05

Status: Implemented; integrated tests and all ten J1–J10 handoff graphs are locally verified.

Plans: [Task0044](../plans/0044-productive-project-crews.md) and
[Task0099](../plans/0099-explicit-recipe-handoffs.md) and
[Task0100](../plans/0100-ten-composed-workflow-graphs.md).
Decisions: [ADR0045](../decisions/0045-productive-crews-and-explicit-tools.md)
and [ADR0052](../decisions/0052-explicit-recipe-handoffs.md) and
[ADR0054](../decisions/0054-composed-workflow-recipe-revisions.md).

## Behavior

The selected local provider generates a bounded structured crew plan through a
durable asynchronous planning job. Questions produce `needs_input`; they do not
become a fabricated completed plan. The owner can answer in a new planning job,
edit members, crew name and validated mission limits, and accept a plan into a
prepared mission. Starting is separate.
Planning includes bounded, explicitly partial accepted-decision context and
selected same-Project terminal-file previews. Accept verifies those file hashes
before freezing the mission inputs; changed sources are rejected.

V11 records workflow facts without rewriting V10 mission/roster bodies. Provider
assignments, job/time/review budgets and immutable subsequent assignments are an
overlay. Owner direction first acknowledges a request, then cancels/removes the
affected work and descendants before replacing it. Unaffected running work and
all previous directories remain. A manual V10 mission can acquire this overlay
through explicit direction. Workflow retries count separately from corrections,
remain inside the job budget and preserve exact owner receipts through a crash.

Agents offer, accept and submit concrete tasks within one Project. Sender identity
comes from the authenticated current run. A submitted reference becomes delivered
only after its accepting run actually completes and the file bytes/hash are read.
Pending offers do not wake closed agents. A delivered `productionSource` resolves
only for an active sender/recipient, rechecks terminal identity and the exact hash,
and supplies private input bytes to production tools. Other jobs remain inaccessible.

Reviewers report concrete conditions using `crew_review`. Failed checks cannot
be accepted. Returns cancel/recreate only the affected dependency chain and copy
the previous completed producer outputs into checked inputs. Required review
acceptance is tied to the current actual producer work item. Text alone does not
complete a required review; exhausting the configured correction/job budget fails
explicitly. Agent guidance explains this requirement and the typed production tools.

The catalog now contains ten bounded J1–J10 compositions plus the unchanged
legacy data recipe. Launch, recording repurpose, study, travel, data briefing,
career package, product campaign, event package, evidence synthesis and meeting
follow-up freeze their exact earlier-member dependencies and final review fan-in.
The meeting flow retains proposed decisions and external app operations behind
the existing owner acceptance and preview/dispatch gates. The older launch,
content, podcast, research, travel and study IDs advance to revision2; persisted
revision1 bodies remain valid rather than being silently rewritten. Owners can
save a Project composition and prepare another mission from it. Recipes retain
roles, settings and any explicit earlier-member dependencies; the legacy data
recipe without that field keeps its original phase derivation. Provider
assignments are configured on the prepared mission. New
recurring local schedules freeze the owner's ready Codex, Claude Code or Gemini
CLI selection; legacy schedules without a provider retain their Codex default.
Durable schedules pause/resume, retain exact occurrence identity, prevent overlap
and skip missed intervals. They run only while the Host is awake, with at most
one new occurrence per tick and at most 100 occurrences.
RFC3339 schedule inputs are normalized to UTC milliseconds; due checks compare
instants. Replay binds saved/enabled schedules to the exact owner payload and
frozen recipe. Each occurrence checks due time, terminal previous mission,
deterministic identity, interval and occurrence budget. A return review authorizes
one exact current-task correction and its descendants within the review budget.
Owner preparation and scheduled occurrences derive the same exact graph. Saving
a prepared/current mission preserves its order and dependencies, while replay
recomputes the graph from the selected frozen recipe revision rather than
trusting arbitrary prepared tasks.

## Bounds and state

Missions retain the existing six-member DAG and concurrency of one to three.
Workflow limits are 1–36 allocated jobs, 1–240 minutes (also constrained by the
mission timeout), and zero to two correction rounds. Selected/copied inputs remain
eight files/25 MiB. Planning uses the adapter's bounded tool-free structured
generation. Provider-reported usage is optional: missing token/cost data remains
unknown, repeated cumulative reports are not summed repeatedly, and there is no
claimed hard token/cost cap. Elapsed time is observed mission wall time, not CPU
time. Whole-mission deadlines include time while the application is asleep.

Full goals, plans, source previews, offer instructions and review bodies stay in
scoped local reads. Routine snapshots use workflow metadata. New facts do not
widen the operational Cloud serializers. Older versions cannot open V11 logs;
rollback requires an unchanged stopped pre-upgrade copy.

## Verification

Eleven workflow tests use real native fake-provider child processes, authenticated
loopback MCP and real files. They cover model questions/cancel/accept, exact
receipts and crash prefixes, immutable selected inputs, affected-only direction,
bounded review/correction with retained originals, recurring pause/resume/restart,
cross-mission offer/delivery and verified delivery consumption by a production
tool. Active MCP document creation produces an actual DOCX whose file/header/hash
are checked; nonce changes and foreign-Project reads are rejected. V1/V2 Cloud
canaries exclude the selected private source content.

The final focused workflow run passes all eleven tests with local loopback MCP
enabled. Added regressions cover configured V11 agent messages through HTTP and
restart, edited model-plan names/limits, schedule offset normalization and altered
owner facts, early/overlapping/forged occurrences, and altered/reused review
corrections. Sandboxed runs without a loopback listener cannot exercise managed
provider starts and do not count as transport evidence.

All 23 tests selected by `cargo test --offline -p patchfleet-core --lib project_`
pass, including the six prior crew tests and coordinator Studio/Project fixtures.
The native fixture uses the production-equivalent five-second probe deadline;
there is no added retry or relaxed production deadline.

Task0099 adds focused proof for the explicit graph. The six-member J5 native
peer run waits for every upstream completion, copies only checked completed
files through hash-pinned manifests, preserves the exact graph through save and
schedule preparation and rejects replay tampering. Recipe-boundary validation
rejects malformed, duplicate, self, forward and out-of-range dependencies.
Turkish/English light/dark native tests keep the handoff summary and primary
action reachable at the minimum window. The standard Rust run passed all 134
core library tests and every core integration binary; the affected desktop
package passed 291/291 with all features after its dynamic-name localization
expectation was corrected. Strict workspace Clippy passes for all targets and
features with warnings denied, formatting passes, and all 153 legacy parity
tests pass on Node 24.22.1. The combined locked workspace command exits cleanly.

Task0100 adds exact catalog assertions for every J1–J10 ID, revision, ability
sequence and dependency matrix. One parameterized native-peer test prepares,
configures, starts and completes all ten graphs, proves every dependent
allocation follows its checked upstream completions, rechecks hash-pinned input
manifests and verifies saved reuse preserves the exact graph. The focused run
passes with local loopback MCP enabled. Native Turkish/English light/dark tests
also prove every visible built-in field is localized and the six-member launch
handoff keeps its Prepare action reachable at the minimum 800×560 window.
The [Turkish dark native capture](../screenshots/workflow-recipes-j1-j10-tr-dark.png)
shows the actual bundled catalog, localized career-package handoffs and visible
Prepare action using disposable Project data and no provider turn.
The final locked all-feature workspace run passes 135 core tests with 13
intentional private-fixture ignores, every integration binary and 293/293
desktop tests. Strict locked workspace Clippy passes with warnings denied,
formatting and diff checks pass, and all 153 parity tests pass on Node 24.22.1.

Live inference, account mutations and native Windows/Linux execution were not
performed. Safe result-file operations retain the existing Unix-only support.
Provider adapters, resource/tool formats and external connections have separate
coordinator/provider validation; recipe text alone proves none of those outputs.
