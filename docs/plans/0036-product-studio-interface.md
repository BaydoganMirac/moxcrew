# Task 0036: Product studio interface

Status: Complete; independently reviewed and locally verified (2026-10-15)

## Outcome

Redesign the native desktop and corresponding web product into one cohesive,
distinctive Patchfleet studio. The owner explicitly requested implementation,
premium visual quality, fluid interactions and the existing lightweight desktop.

## Design contract

- Graphite canvas (#101315), slightly raised panels (#191e21), warm white
  text (#eef0e9), muted sage-gray (#9ca8a5), restrained mint (#b6edca).
- A geometric three-stroke fleet mark, clear type hierarchy, quiet separators,
  generous section spacing and compact operational rows. No decorative live
  charts, fabricated work, permanent animations or new rendering framework.
- Group navigation into Workspace (projects/work/teams), Knowledge
  (memory/decisions), and System (agents/providers/cloud). Keep real status
  distinct from commands awaiting application.
- Make first-project setup inviting, project cards useful, and evidence-focused
  screens readable. Hide secondary diagnostics behind existing disclosure.
- Shared visual language, not identical layouts: native 800x560 minimum and
  large windows; web responsive from 360px with keyboard/focus/reduced-motion.
- Preserve native Turkish/English, original owner content, folder pickers,
  explicit consent, account-optional Local and current capability gates.

## Ownership

- Coordinator: this plan/state, native desktop src UI/theme/localization and
  test-only scenes, public docs/release, final integration and visual review.
- Web builder: private Cloud app presentation excluding app/api and
  app/desktop/authorize; private presentation tests and its plan/state only.
- Reviewer: read-only inspection after stable implementation, focused on
  interaction regressions, honest status, responsive layout and safety gates.

No domain schema, provider, persistence, protocol, billing-entitlement or
authentication behavior changes. No deployment, publication, model calls,
owner-data tests or installed-app replacement.

## Acceptance

1. Native all-tab and existing action/consent/revision tests pass; fmt and strict
   Clippy pass. Actual native frames inspected in both languages at normal and
   minimum size, including populated and first-use screens.
2. Web production build/type checking and existing product/security tests pass.
   Local browser verifies public, signed-in populated/empty states and mobile;
   preserve forms, receipts, auth and privacy boundaries using disposable data.
3. Both surfaces use the agreed palette/mark/hierarchy. Interactive controls
   remain functional; no fictitious production activity.
4. Feature-off native build/package, normal close and bounded idle measurement;
   record actual results and platform limits. Update state and commit locally.
