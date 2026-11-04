# Task0069: Exact owner review notes

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to complete the remaining non-Windows product
gaps. Windows runtime proof remains deferred to a Windows PC.

## Problem

An exact artifact can be marked reviewed or needs changes, but the decision has
no explanation. The owner must reconstruct what was checked or what should be
fixed from outside Moxcrew, so the durable review marker is not yet actionable.

## Scope

1. Add an optional bounded owner note to an exact artifact review decision.
2. Persist and replay the note only with the matching Project/work/path/SHA-256.
3. Let the owner edit the note before marking reviewed or needs changes and show
   the saved note in Output Studio.
4. Clearing review removes the active status and note while retaining canonical
   audit history.
5. Keep notes local: do not add them to delivery manifests, Cloud projections or
   unrelated receipts.

## Non-goals

- No automatic review text, semantic evaluation or multi-reviewer approval.
- No provider/model call, task start, file mutation, publication or external
  action.
- No note inheritance across artifact hashes.
- No Windows runtime claim.

## Acceptance

- A new event version preserves V15 replay while validating bounded V16 notes,
  exact authority, recovery, clearing and hash isolation.
- Desktop tests prove localized note editing, exact payloads, saved display and
  minimum-window reachability.
- Delivery tests prove review notes never enter the ZIP manifest.
- Full workspace/all-feature tests, strict Clippy, formatting and diff checks
  pass; material native UI changes receive a retained visual check.
