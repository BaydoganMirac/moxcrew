# Exact artifact owner review

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0067](../plans/0067-exact-artifact-owner-review.md)

Output Studio now lets the owner record a human decision for one exact artifact
version. **Mark reviewed**, **Needs changes** and **Clear review** bind the
selected Project, work item, relative path and SHA-256. The Host rereads the
current bytes before accepting the command; a changed file, foreign Project or
missing work item is rejected without creating a review fact.

The current decision appears beside the existing verification summary. It is
deliberately labeled as owner review, not automatic quality evidence. Format
inspection still says only what Moxcrew reopened or parsed, while the owner
decision says only that this exact version was reviewed or needs changes. A new
hash starts unreviewed and never inherits the prior version's decision.

V15 local events record the exact review scope and owner command/receipt. Exact
retries return the original receipt; restart rebuilds the decision from the
canonical log. Clearing appends an auditable `unreviewed` transition and removes
the active decision from the projection without deleting history or the file.
No provider, model, task start, external app or Cloud action is involved.

[Task0069](exact-owner-review-notes.md) extends this exact decision with an
optional bounded local note. V16 preserves V15 replay, and clearing removes the
active note with the status. Notes are not exported in delivery manifests.

The Turkish/English native card is covered in light/dark themes and remains
scroll-reachable at the 800×560 minimum window. The retained Turkish dark-theme
capture is
[artifact-owner-review-tr-dark.png](../screenshots/artifact-owner-review/artifact-owner-review-tr-dark.png)
(2560×1760, SHA-256
`7fc6420e60c25ae2de6a9d37181ae29884d4bd474f58fe3fbb1a2ab2d67c6896`).

## Verification

- Core behavior covers exact scope, Project isolation, changed-byte rejection,
  idempotent recovery, version independence, clearing and restart replay.
- Desktop behavior covers exact command payloads, localization, themes,
  minimum-window reachability and foreign-version isolation.
- `cargo test -p patchfleet-desktop --all-features`: 245 passed.
- `cargo test --workspace --all-features -- --test-threads=1`: 540 passed;
  6 opt-in external-environment tests intentionally ignored.
- Strict workspace Clippy, formatting and diff checks pass.

## Limits

This is an owner-authored review marker, not semantic evaluation, factual
verification, rendered-layout proof or media playback evidence. Multi-reviewer
approval and Windows result-access proof are not included.
