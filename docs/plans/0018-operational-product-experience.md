# Task 0018: Operational product experience

Status: Approved

Owner approval: 2026-07-18

## Goal

Turn the paired Local and Cloud beta into one understandable product flow:
Local queues and controls work, while Cloud answers what needs attention and
proves the outcome of the one supported remote action.

## Scope

- Recover safely when a changed sanitized projection conflicts with the last
  successful projection at the same local event count.
- Make Local lead with active work, quick task creation, and recent outcomes;
  keep first-run guidance conditional and provider sessions diagnostic.
- Make Cloud lead with host freshness, active work, the supported cancel
  action, and its terminal receipt; make pairing dominant only before the first
  host exists.
- Explain delayed, stale, offline, retrying, and rejected states in plain
  language without exposing source, prompts, output, credentials, or paths.
- Improve the failed authentication experience while preserving login rate
  limits and strict request validation.
- Cover returning, first-use, paired, stale, mobile, and error states in Local
  and Cloud tests.

## Acceptance criteria

1. A returning Local owner sees active work and task creation in the first
   viewport; completed onboarding and provider session details do not dominate.
2. A paired Cloud owner sees reachability, data freshness, current work, the
   available safe action, and recent outcome before setup or diagnostics.
3. Projection changes remain monotonic across application upgrades; equal
   conflicting content can recover once, while genuinely stale data still
   fails closed.
4. Login infrastructure failures return actionable page feedback instead of a
   raw internal-error response.
5. Both repositories pass their test suites, production builds, responsive
   browser review, and diff checks.
6. The live remote cancel is considered successful only after Local returns a
   definitive terminal receipt and Cloud displays it.

## Boundaries

- No new dependency, framework, design system, analytics, billing, remote
  command, arbitrary shell, provider execution support, or protocol field.
- Cloud remains a sanitized, rebuildable projection and typed intent service;
  Local remains canonical and fully usable offline.
- Approval covers code, tests, documentation, commit, and push. It does not
  authorize a live database migration, service deployment, DNS change, npm
  publication, or external invitation.
