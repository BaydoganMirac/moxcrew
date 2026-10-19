# General-purpose Project direction

Status: Owner-selected product direction recorded; broader execution pending

Updated: 2026-10-19

## Accepted direction

[ADR 0038](../decisions/0038-general-purpose-projects.md) records the owner's
choice: continue with Project rather than a Workspace approach, and support
people doing general work rather than targeting only software developers.
Research, writing, analysis, planning and personal work are representative
future outcomes. A Project is the stable scope for context, tasks, results and
owner-accepted decisions; it is not synonymous with a repository.

## Recorded this session

Updated the public product promise, README, roadmap and decision index. Aligned
the private Cloud strategy with the same public direction. The previous
developer-only positioning is superseded; canonical Local ownership, explicit
Cloud sharing and free Local/optional subscription Cloud are preserved.

## Current implementation and next step

Existing project identity and owner-entered memory are independent of folders.
Task/team execution and the delivered Quick start still use Git worktrees and
supported Codex control. No non-coding runner, file import, browser/application
control, general preference inheritance, migration or new onboarding shipped
in this clarification. Historical internal workspace IDs remain compatible.

The owner subsequently rejected the narrow three-ability launch proposal and
selected a broad release including presentations, video editing and research.
[ADR 0039](../decisions/0039-broad-general-purpose-ability-release.md) records
this direction. Detailed capabilities, skill/tool compatibility and execution
environment remain to be scoped in bounded cards within one release. No broader
runtime, end-user validation, universal skill compatibility, push or deployment
is implied by this documentation change.

The owner then accepted the broad plan for version 1.0.0.
[ADR 0040](../decisions/0040-version-one-release-baseline.md) and the
[master plan](../plans/0039-version-one-release.md) now define the eight-area,
forty-ability, ten-workflow baseline. Continue using
[release state](version-one-release.md) for concrete implementation progress.
