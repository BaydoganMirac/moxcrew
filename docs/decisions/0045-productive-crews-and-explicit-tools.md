# ADR 0045: Productive crews with explicit local tools

Status: Accepted by owner-approved Task0044

Date: 2026-10-19

## Decision

Implement the twelve accepted features on the Project conversation and reusable
crew foundation. Local provider inference produces inspectable plans; planning
does not authorize execution or external side effects. Prepared missions keep
immutable inputs and explicit start. Workflows can direct affected work, delegate
within a Project, review outputs and schedule bounded occurrences.

Preserve existing local event meanings. V11 records workflow commands/facts; V12
records frozen resources, artifacts and prepared app operations; V13 explicitly
extends work/run execution to Claude Code and Gemini CLI alongside Codex. Their
provider differences stay behind adapters. Existing V1 work remains Codex-only.
New local details do not widen public Cloud serializers or remote intents.

Use owner-installed Python 3 for bundled standard-library Office/data scripts and
FFmpeg/FFprobe for deterministic media transforms. Tools run as bounded child
processes in scoped job directories through typed operations, never user-supplied
shell commands. They are optional dependencies with explicit setup/preflight;
the Rust native app and single Host remain the only persistent runtime. Optional
document renderers are capability checked independently. Missing tools produce
an actionable unavailable state rather than an invented output.

The output surface renders supported local previews and supports scoped revisions
which create new versions. Import only selected resource files with size/type/hash
checks. Agent reads derive Project identity from the active run. Resource and skill
text cannot grant permissions or overwrite accepted Project decisions.

Initial app operations are Google Drive file import, Notion page creation and
Google Calendar event creation through fixed documented HTTPS APIs. Owners supply
account access through native private configuration; secrets do not enter events,
snapshots, prompts or logs. Prepare a concrete operation before explicit approval.
Persist dispatch before network writes and distinguish uncertain outcomes from
success; never auto-repeat an uncertain non-idempotent write.

Recurring work uses explicit owner authorization, a durable next time and bounded
run count/time. A local host must be awake; skip missed intervals and prevent
overlapping occurrence launches. Display actual usage when the provider reports
it; absent measurements stay unknown and cannot enforce a claimed hard token cap.

## Consequences

This expands output and provider coverage without introducing a WebView, Node
sidecar, workflow database or Cloud execution. Optional tool installation and
account access remain visible setup costs. Compatibility requires an unchanged
stopped backup before adopting logs with newer event versions. Validation uses
real local outputs and deterministic provider/network fixtures; public release
claims additionally require real provider/account and supported OS evidence.
