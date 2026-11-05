# Task0099: Explicit recipe handoffs

Status: In progress

Updated: 2026-11-05

Authorization: the owner approved continued closure of the remaining Moxcrew
product gaps, asked for the complete desktop experience to work before Windows
verification, and confirmed that no deletion may happen without a separate
question. This card does not delete files, call a live provider/account, publish,
push, tag or release.

## Problem

Prepared missions already support checked task dependencies and copy completed
upstream files into the next task through a hash-pinned input manifest. Reusable
recipes, however, preserve only their roster and settings. Preparation therefore
reduces every recipe to research, parallel producers and final review. A real
composition such as data cleaning, analysis, report, presentation and audio brief
cannot retain its ordered handoffs, and saving that crew loses its workflow.

## Scope

1. Add one bounded, optional recipe member-dependency plan. Dependencies refer
   only to earlier members, remain acyclic by construction and never authorize
   execution.
2. Convert the frozen recipe plan into the existing mission `dependsOn` graph for
   explicit owner preparation and scheduled occurrences. Keep the existing
   prepare/start boundary, budgets, provider assignments and input checks.
3. Preserve the exact task graph when an owner saves a prepared/current mission
   as a reusable recipe.
4. Add a bundled data-briefing recipe for the J5 composition: supplied data to clean
   data, checked analysis, charts/report, editable presentation, audio brief and
   final review.
5. Show the selected recipe's ordered handoffs in the native recipe detail without
   adding another primary action or hiding required setup information.

## Acceptance

- Missing, malformed, forward, duplicate, self or out-of-range dependency indices
  fail closed; legacy recipes retain their existing phase behavior.
- Owner recipe preparation and schedule preparation produce the same frozen task
  graph, and event replay verifies the graph against the selected recipe revision.
- A saved recipe reproduces the source mission's task order and dependencies.
- The J5 recipe has six bounded members and the exact chain clean -> analyze ->
  report/charts -> presentation -> audio brief, with final review depending on all
  five producers.
- Runtime tests prove each downstream task waits, receives only checked completed
  upstream files and still supports bounded targeted correction. Native tests
  prove the flow is understandable and the primary action remains reachable at
  800x560 in Turkish/English light/dark themes.
- Focused tests, repository gates, state documentation and one retained native
  screenshot pass; the final local commit contains only intended files.

## Limits

This task proves deterministic orchestration and file handoff. It does not claim
that a live model produces a good analysis, presentation or voice, does not add
automatic data interpretation, does not install an engine, and does not perform
an external app action. Individual production operations keep their existing
automatic and human-review boundaries. Windows-native and real-provider quality
verification remain deferred.
