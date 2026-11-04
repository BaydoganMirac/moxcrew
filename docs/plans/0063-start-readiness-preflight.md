# Task0063: Start readiness preflight

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: After reviewing the rebuilt interface, the owner asked to verify
the system and implement missing features. Windows work remains deferred to a
Windows PC; this card closes the non-Windows gap between visible ability
readiness and explicit task/crew start.

## Problem

Task0062 shows declared local prerequisites while choosing an ability, but a
queued Project job or prepared crew can still be started from several native
surfaces without seeing that result. Catalog guidance must survive into the
last explicit action before work begins.

## Scope

1. Preserve the originating ability ID in the existing local `projectJobs`
   runtime summary for direct, retried, crew and Studio work without adding a
   canonical event or Cloud field.
2. Reuse the existing Project Studio readiness read and Task0062 profiles for
   queued jobs and prepared crews.
3. Apply one consistent preflight to Chat, Create jobs, Crew and Tasks start
   actions. Missing required local tools, image-engine or app setup prevents the
   native start action and offers a route to the existing setup surface.
4. Keep source, browser, voice/model and awake-Host access visible as a start-time
   check, not a locally proven blocker. Optional tools never block.
5. Cover Project isolation, retry/crew ability provenance, all start surfaces,
   setup routing and minimum-window behavior with tests.

## Non-goals

- No automatic installation, login, external action or model request.
- No claim that a passed preflight proves output quality or provider entitlement.
- No new durable schema, Cloud projection or command authority.
- No Windows file handling, packaging or runtime verification.

## Acceptance

- Every managed Project job that has a declared ability exposes only that stable
  ability ID in the local snapshot; unrelated/private request content does not.
- Start remains enabled for ready local checks and access-only checks, and is
  disabled while required local checks are unknown or missing.
- The user can open the exact existing Tools, Images or Apps setup surface from
  every blocked start context.
- Focused and full serial workspace tests, strict Clippy, formatting and diff
  checks pass. State documentation and a local review commit are current.
