# Task0046: Product crew design

Status: Complete; independently reviewed and locally verified

Updated: 2026-10-19

The owner rejected Task0045's visual design and explicitly requested an actual
product design implemented in the app. This is a native presentation correction,
not a new runtime or Cloud capability.

## Observable result

- One compact Project header combines the project picker and Chat / Crew /
  Outputs switch. No stacked equivalent navigation rows.
- Responsive navigation reserves useful content width on small windows.
- Crew is a designed work surface: a short goal summary, distinguishable agent
  cards, real role/ability information, and one clearly framed next action.
- Direction, recipes, schedules and activity remain reachable through contextual
  controls. The default plan does not show five secondary tabs at equal weight.
- Request forms and technical planning metadata are secondary. Prepare remains
  separate from Start. All existing command, draft, file and approval scopes stay.
- Output Studio follows the same visual language and retains exact file-version
  preview/revision/export and explicit prepared app actions.
- Actual native TR/EN light/dark screenshots at 800x560 and normal width, plus
  interaction tests, must prove usable layout. Independent review judges the
  visual result as well as routes. Deliver a separate feature-off development
  package, without public release or push.

## Ownership

Coordinator owns chat.rs, ui.rs, studio.rs, task/docs, smoke fixture copy in
smoke.rs if applicable, integration/captures/package. Builder owns crew.rs,
output_studio.rs, create.rs and i18n.rs. Coordinate shared helpers before use.
Reviewer is read-only after a stable commit and actual screenshots exist.

No new dependencies, WebView, schemas, provider calls or external account writes.
Keep exact receipt scopes, private token clearing, explicit frozen approval,
keyboard/focus accessibility and reduced motion. Do not describe mock fixtures
as live agent work or infer dependencies not present in the actual plan.

## Completion

The redesigned native surface, responsive keyboard-focus correction,160passing
all-feature desktop tests, strict workspace Clippy/formatting,52finalnative
captures and separately verified feature-off development package are recorded
in [product crew design state](../state/product-crew-design.md). Runtime and
Cloud contracts did not change. Owner aesthetic feedback and public/other-platform
release validation remain separate from the completed implementation.
