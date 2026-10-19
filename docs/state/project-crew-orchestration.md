# Project crew orchestration

Updated: 2026-10-19

Status: Complete; independently reviewed and locally verified.

Plan: [Task0043](../plans/0043-project-crew-orchestration.md).
Decision: [ADR0044](../decisions/0044-project-crews-and-scoped-communication.md).

## Behavior

Chat defaults to an inspectable local crew recommendation with editable members,
role instructions and bounded limits. Preparing creates a durable mission;
starting is a separate owner command. Saved crews are reusable within their
Project. Single task retains the existing conversation and selected-file follow-up.
Teams is a primary navigation item; legacy coding teams remain accessible there.

Crew missions reuse actual managed Codex jobs and the existing single Host writer.
Dependencies receive checked terminal output copies. The native crew panel shows
member task states, same-Project messages and actual job files. Sender/run identity
is derived by the Host. Pending messages do not wake closed agents or imply replies.
Limits: six members, concurrency one to three, zero to two retries, one to 120
minutes, eight copied files/25 MiB, bounded 2,000-byte messages, 500 sent messages
per mission and 50-item pages.

## Persistence and boundaries

V10 records roster/mission preparation, lifecycle, task allocation and messages.
Existing V1/V2/V8/V9 meanings and Cloud serializers remain unchanged. Immutable
input manifests and fresh task directories preserve prior artifacts. Restart
retains acknowledged outcomes; interrupted runs require explicit recovery/retry.
Full messages stay outside routine runtime snapshots and all crew metadata stays
outside operational Cloud payloads. Older applications cannot open V10 logs;
rollback needs an unchanged pre-upgrade copy made while the Host is stopped.

## Validation

The offline locked workspace suite with all features passes: 354 tests, zero
failures, four existing opt-in tests ignored. These exclusions are the installed
Codex wrapper probe, two private PostgreSQL integration fixtures and the large-log
timing probe. The 139 desktop checks include exact asynchronous crew receipt
scope, preserved drafts/rosters, actual file scope and prepare without execution.
The final label-only correction also passes all ten focused crew UI tests.
Formatting and strict workspace all-target/all-feature Clippy pass.

Six focused runtime tests launch fake supported CLI processes and use real
loopback MCP HTTP to exercise the dependency graph, actual output handoff, same-Project cross-mission
messages, duplicate requests, cancellation, retries, restart and owner receipt
crash prefixes. Tests reject foreign Projects, stale/forged sender runs, malformed
commands, altered immutable allocations and tampered inputs. Separate boundary
checks cover frozen prior-file references and byte-identical V1/V2 Cloud canaries.
Old persistence fixture shapes and hashes still pass.

Forty-two actual native GL frames cover plan, roster, running crew, messages and
Teams in TR/EN, light/dark, 1280x880 and 800x560, plus minimum welcome/connections.
Visual inspection caught a missing Turkish tab label and unsupported arrow glyph;
sixteen affected frames were recaptured at `b67f54b`. The captures use disposable
local data and explicit rendering fixtures, with no provider turn or fabricated
production output.

- [Crew plan](../screenshots/project-crews/crew-plan-tr-dark.png)
- [Minimum crew plan](../screenshots/project-crews/crew-plan-en-light-min.png)
- [Minimum editable roster](../screenshots/project-crews/crew-roster-en-light-min.png)
- [Running crew panel](../screenshots/project-crews/crew-running-tr-dark.png)
- [Attributable messages](../screenshots/project-crews/crew-messages-tr-dark.png)
- [Minimum messages](../screenshots/project-crews/crew-messages-en-light-min.png)
- [Reusable Teams](../screenshots/project-crews/crew-teams-tr-dark.png)

Independent review approved `b67f54b` (runtime integration `9544d1f`) with no
remaining confirmed P1/P2 findings. It verified scoped authorization/deadlines,
immutable inputs, receipt recovery, draft/file isolation and native usability.
Live inference, Windows runtime and package signing were outside that review;
the coordinator separately verified the macOS development package below.

## Development package

The feature-off release build at `b67f54b` passes. The macOS ARM64 App is ad-hoc
signed, without public notarization. Strict signature, bundle identity/version,
license files and absent smoke-control checks pass. An isolated NSWorkspace
launch with temporary data and failing provider stubs displayed its own native
window, closed normally and released the writer lock. No installed application
or owner data was replaced. Native Windows/Linux execution is not verified.

`dist/native/project-crews/Moxcrew_0.12.0_crews_aarch64.dmg` is 7,778,844 bytes;
SHA-256 `a8f52255deea1e47b252ad60fdb3e98afd7a1e44978eb746d7a3d7faa6517d18`.
The packaged executable is 14,956,992 bytes; SHA-256
`04f58c3d5fc8c09e14b69cc417d4c8f043c3ef0fb47be4903a9d165bbe791ead`.
DMG integrity verification passes. The six-second empty launch sampled
112,848 KiB RSS and 0.14 seconds cumulative CPU; this is a startup observation,
not a sustained performance guarantee.

## Limits

Execution uses supported local Codex control. Claude/Gemini connection diagnostics
remain available; this change adds no execution adapter for them. Recommendations
use bundled guides and explicit goal terms, not a model-generated plan. A recipe
or crew does not install an external renderer or prove an ability's output quality.
Messages to a closed agent remain pending until that agent actually reads them in
an authorized run. Native output file access retains the existing Unix-only safe
handle support; Windows result access remains unavailable. No Cloud deployment,
push, provider login, API key or live inference is needed for this implementation.
