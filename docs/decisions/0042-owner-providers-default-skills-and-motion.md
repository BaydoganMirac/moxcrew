# ADR 0042: Owner-connected providers, default skills and expressive motion

Status: Accepted product requirements; runtime implementation pending

Date: 2026-10-19

## Decision

Mox coordinates work through the user's local Codex, Gemini CLI or Claude Code
installation. The owner reaffirmed these three integration targets. At least
one supported provider must be connected before inference work; installing or
connecting all three is optional. A Moxcrew Cloud account is not the provider
connection. Local CLI execution does not imply on-device model inference.
Provider accounts, data transfer and usage charges retain their own terms.

First use must detect installed providers, guide missing installation/sign-in,
and check supported version, authentication and task/tool capability separately.
A version check alone cannot mean connected or ready. Use supported structured
checks; when authentication cannot be verified without inference, disclose that
and offer an explicit bounded test with its possible usage cost. Never inspect,
copy or send provider tokens to Cloud. Do not silently switch providers or send
the owner's goal to another provider after a failure.

The default experience must include useful, maintained skills across the
existing eight-area, forty-ability and ten-workflow 1.0.0 baseline. A default
skill includes inspectable instructions, compatible tools, templates when
needed, input/output rules, failure handling and actual output validation.
Existing skills can be reused with verified provenance, license and pinned
version. A catalog entry or SKILL.md alone does not satisfy delivery.

Before work, report required tool/model/account access and missing dependencies.
Installation must be deliberate and scoped; a skill grants no new permissions.
Verify real outputs and targeted revisions: research claims against sources,
documents/slides by reopen/render, media by decode/timing, and calculations
independently. Record support per provider and ability; a connected CLI does
not automatically supply image, speech or every external tool capability.
Mox selects suitable available skills from the goal; manual selection remains
an override, not required skill-system knowledge.

Use expressive brand motion for connecting, active crew work, handoff and
artifact arrival. Mox reacts to actual state. Do not invent agents, tool calls,
percentages or success to decorate a waiting state. Keep motion brief, respect
reduced-motion preferences and keep idle rendering sparse under ADR0024.
The current vector Mox identity and native renderer remain the foundation.

## Chat interaction

A project-scoped conversational surface is the proposed interaction direction:
goal and follow-up in the center, actual files in an on-demand side panel, and
progress tied to durable work. Sources, decisions, revisions and cancellation
must retain exact project/job scope across restart and provider changes.
The owner's conditional chat feedback establishes the quality requirements
above; it does not prove persisted chat, streamed responses or follow-up context
already exist. Detail that implementation in a bounded card before runtime work.

## Delivery truth

At this decision's scope, the controller supported only Codex execution and the
other providers were observation-only. The later
[provider execution state](../state/provider-execution.md) records bounded
Claude/Gemini control that supersedes that limitation. Create's forty recipes
still must not be confused with forty independently quality-proven skills.

No provider login, credential creation, skill/tool installation, protocol
extension or runtime change follows from recording these requirements.

## References

- [1.0.0 master plan](../plans/0039-version-one-release.md)
- [Provider adapters](../../.agents/skills/provider-adapters/SKILL.md)
- [Native runtime](0024-native-rust-desktop-ui.md)
- [Release state](../state/version-one-release.md)
