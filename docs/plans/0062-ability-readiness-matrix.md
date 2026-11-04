# Task0062: Ability readiness matrix

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product gaps
and specifically to continue implementing missing features after the interface
work. Windows runtime verification remains deferred to a Windows PC.

## Problem

The Create catalog exposes forty inspectable recipes, while live production-tool
readiness is visible only after navigating to Output Studio. A user can choose a
recipe without seeing which installed tools or separately authorized access the
recipe may need. The existing generic disclaimer is accurate but not actionable.

The catalog must not turn recipe metadata into a quality or live-model claim.
Readiness means only that the declared local prerequisites are present. Source,
voice/model and recurring-work access remain explicit checks at start time.

## Scope

1. Give every release ability an exhaustive, typed readiness profile containing
   required local tools, optional enhancement tools and separately checked access.
2. Combine that profile with the existing Project Studio readiness response in
   the native Create catalog. Never probe or install a tool from the render thread.
3. Show one compact, localized status and a plain-language requirement summary
   before an ability is selected.
4. Route missing tool, image-engine and supported-app prerequisites to their
   existing setup surfaces. Selecting a recipe remains allowed and never starts
   work.
5. Cover all forty IDs, live/missing/unknown readiness states, command dispatch,
   setup routing and minimum-window visibility with behavior tests.

## Non-goals

- No new production engine, provider router, background service or canonical event.
- No claim that installed tools prove output quality, live provider entitlement,
  browser source access or voice generation.
- No automatic install, external action, task preparation or task start.
- No Windows result-access work or Windows runtime evidence in this task.

## Acceptance

- The core catalog test proves all forty abilities have valid profiles and only
  known readiness identifiers.
- Opening the catalog for a Project requests the existing bounded Studio read once
  and remains usable while that read is pending.
- Loaded Project state distinguishes local checks ready, setup needed and access
  checks without treating optional tools as blockers.
- Setup actions open the exact existing Tools, Create outputs or Apps view.
- Turkish and English labels render at 800×560 without hiding Close or the setup
  action.
- Focused tests, the full workspace/all-feature suite, strict Clippy, formatting
  and diff checks pass. The relevant state and roadmap are current and a local
  commit is ready for owner review.
