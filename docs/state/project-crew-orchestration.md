# Project crew orchestration

Updated: 2026-10-19

Status: Implementing; validation and independent review underway.

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
minutes, eight copied files/25 MiB, bounded 2,000-byte messages and 50-item pages.

## Persistence and boundaries

V10 records roster/mission preparation, lifecycle, task allocation and messages.
Existing V1/V2/V8/V9 meanings and Cloud serializers remain unchanged. Immutable
input manifests and fresh task directories preserve prior artifacts. Restart
retains acknowledged outcomes; interrupted runs require explicit recovery/retry.
Full messages stay outside routine runtime snapshots and all crew metadata stays
outside operational Cloud payloads. Older applications cannot open V10 logs;
rollback needs an unchanged pre-upgrade copy made while the Host is stopped.

## Validation

Core/native compile checkpoint passed. Full workspace, targeted lifecycle/MCP,
TR/EN light/dark/minimum-window rendering, strict Clippy, independent review and
feature-off packaging are in progress. Final evidence will replace this section.

## Limits

Execution uses supported local Codex control. Claude/Gemini connection diagnostics
remain available; this change adds no execution adapter for them. Recommendations
use bundled guides and explicit goal terms, not a model-generated plan. A recipe
or crew does not install an external renderer or prove an ability's output quality.
Messages to a closed agent remain pending until that agent actually reads them in
an authorized run. Native output file access retains the existing Unix-only safe
handle support; Windows result access remains unavailable. No Cloud deployment,
push, provider login, API key or live inference is needed for this implementation.
