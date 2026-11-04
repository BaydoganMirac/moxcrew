# Exact revision comparison

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0066](../plans/0066-exact-revision-comparison.md)

Output Studio now makes a preserved parent practically reviewable without a
model turn, durable schema or remote action. **Compare with previous version**
appears only when the selected artifact has a `parentVersionId` whose exact hash
remains in the selected Project's current Studio listing. A revision may use a
new filename without losing its hash-pinned parent relationship.

The owner action emits one existing read-only `project_artifact_read` for the
parent. Moxcrew keeps a bounded text/Office-structure excerpt in an ephemeral
eight-entry cache, then issues one exact read to restore the selected current
preview. Project selection, artifact selection and source files do not change;
no work is queued or started.

The responsive comparison shows both hashes as exact versions. Text is reopened
directly; PPTX/DOCX-style pages are flattened only as structural headings and
text. Copy is capped at 12,000 characters per cached version and rendered as a
short first-look excerpt, with the bounded full excerpt available on hover.
Image pixels, media playback, meaning, factual correctness and rendered Office
layout are explicitly outside the claim.

All comparison material clears when the Project changes, disappears from the
current Studio listing or the Host epoch changes. Foreign Project/work/path/hash
responses are never cached. The native fixture creates two real preserved text
files through the revision queue without starting a provider and renders the
result in [revision-comparison-tr-dark.png](../screenshots/revision-comparison/revision-comparison-tr-dark.png)
(2560×1760, SHA-256
`b3cf5a65270b806a2b93d875e8497f280ceb4706c362d304d7f9f806677724cd`).

## Verification

- Focused desktop comparison tests cover exact parent/current command scope,
  automatic current-preview restoration, selection preservation,
  localization, themes, minimum-window reachability, cache bounds and
  Project/Host invalidation: 2 passed.
- `cargo test -p patchfleet-desktop --all-features`: 244 passed.
- `cargo test --workspace --all-features -- --test-threads=1`: 538 passed;
  6 opt-in external-environment tests intentionally ignored.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo fmt --all -- --check` and `git diff --check` passed.

## Limits

The parent must still be present inside the bounded current Studio listing.
Comparison is unavailable for images, media and unsupported formats. This does
not provide semantic diff, merge, output acceptance, full quality review or
Windows result-access evidence.
