# Task0068: Review-aware delivery packages

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to complete the remaining non-Windows product
gaps. Windows runtime proof remains deferred to a Windows PC.

## Problem

Output Studio records an owner decision for one exact artifact version, but the
package-selection flow and exported ZIP manifest do not carry that context. An
owner can therefore deliver an unreviewed or needs-changes version without a
clear warning, and a recipient cannot tell which packaged bytes had been marked
reviewed when the archive was created.

## Scope

1. Show exact-version review status for every file in package selection.
2. Summarize selected reviewed, needs-changes and unreviewed file counts and
   warn without silently excluding or replacing files.
3. Export the current exact-version owner-review status beside every file in a
   versioned ZIP manifest.
4. Preserve explicit owner control: review state informs but does not block a
   deliberate export.
5. Keep delivery receipts backward-compatible and avoid adding review metadata
   beyond the explicit exported archive.

## Non-goals

- No automatic quality score, semantic verification or approval workflow.
- No provider/model call, task start, file mutation, publication or external
  action beyond the owner's existing explicit ZIP export.
- No review inheritance across artifact hashes.
- No Windows runtime claim.

## Acceptance

- Core tests prove manifest status for reviewed, needs-changes and unreviewed
  exact versions without changing the delivery receipt contract.
- Desktop tests prove per-file status, selected coverage, warnings, localization
  and minimum-window reachability while export remains owner-controlled.
- The full workspace/all-feature suite, strict Clippy, formatting and diff checks
  pass; native rendering evidence is captured for the material UI change.
