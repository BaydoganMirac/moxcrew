# Task0067: Exact artifact owner review

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product gaps
after the interface and system-readiness work. Windows runtime proof remains
deferred to a Windows PC.

## Problem

Output Studio proves the selected file hash, reports bounded format inspection
and names the remaining human review. It does not let the owner record that
review. A later session therefore cannot distinguish an exact version the owner
reviewed from one that still needs changes.

## Scope

1. Add a durable owner command for one exact Project/work/path/SHA-256 tuple.
2. Record either `reviewed` or `needs_changes`; clearing the decision returns the
   exact version to `unreviewed` without changing or deleting the artifact.
3. Show the decision in Output Studio only for the matching exact version.
4. Keep this explicitly human evidence: format inspection, owner review and
   model/output quality remain separate claims.
5. Preserve idempotency, restart recovery, Project isolation and stale-hash
   rejection through a new local event version.

## Non-goals

- No automatic quality score, semantic verification or rendered-layout claim.
- No provider/model call, task start, file mutation, publication or external
  action.
- No inheritance of a review decision by a later artifact version.
- No Windows runtime claim.

## Acceptance

- Core tests prove exact hash scope, Project isolation, idempotent recovery,
  stale-byte rejection and independent decisions across versions.
- Desktop tests prove exact command payloads, clear/review/change states,
  localization and minimum-window reachability.
- The full workspace/all-feature suite, strict Clippy, formatting and diff checks
  pass; native rendering evidence is captured if the UI changes materially.
