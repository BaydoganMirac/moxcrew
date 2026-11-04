# Task0066: Exact revision comparison

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product
gaps. Windows stays deferred to a Windows PC. This slice makes preserved output
versions practically reviewable without a model turn or remote account action.

## Problem

Output Studio lists a revision's exact `parentVersionId`, but an owner must
manually switch between files and remember the content. The application does not
offer a scoped comparison of the current output and its preserved parent.

## Scope

1. Resolve a parent only when its exact SHA-256 version is in the selected
   Project's current Studio listing, even if a revision used a new filename.
2. Load the previous version only after an explicit owner action, using the
   existing read-only `project_artifact_read` command.
3. Keep a small ephemeral, host-epoch-scoped cache containing only bounded text
   or Office-structure comparison copy.
4. Restore the selected current preview after the parent read and show both
   exact versions in one responsive comparison surface.
5. State that structural/text comparison is not semantic correctness or
   rendered-layout proof; localize and test the minimum window.

## Non-goals

- No semantic diff, quality score, merge, acceptance or automatic revision.
- No durable event/schema, background read, model turn or remote app action.
- No comparison claim for image pixels, media playback or unsupported formats.
- No Windows result-access claim.

## Acceptance

- Foreign Project/work/path/hash responses never enter comparison state.
- Loading the parent emits one exact read and then restores the current exact
  read without changing the selected artifact or starting work.
- Switching Project or Host epoch clears all cached comparison material.
- Turkish/English and light/dark minimum-window tests plus a native revision
  fixture demonstrate the complete flow.

## Verification

- Focused desktop comparison: 2 passed.
- Full desktop package: 244 passed.
- Full serial workspace: 538 passed; 6 opt-in external-environment tests
  intentionally ignored.
- Strict workspace Clippy, formatting and diff checks passed.
- Native Turkish dark-theme revision capture reviewed at 2560×1760.
