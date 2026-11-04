# Exact owner review notes

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0069](../plans/0069-exact-owner-review-notes.md)

Output Studio now gives each exact artifact review an optional owner note. The
note can record what was checked or what should change before the owner marks the
version **reviewed** or **needs changes**. It is bound to the same Project, work
item, relative path and SHA-256 as the decision; another version never inherits
the text.

The Host accepts at most 4,000 canonical UTF-8 bytes (the UI allows 1,000
characters), rejects padded, NUL-containing or oversized values, and rereads the
exact artifact bytes before appending the decision. Clearing review requires an
empty note, removes both active values from the projection and retains the
append-only audit history.

V16 extends the local owner command/fact/receipt with the bounded note. V15
review facts continue to validate and replay with an empty note. Recovery after
a missing receipt returns the exact V15 or V16 result, while V12/V15 schema
isolation rejects the V16 note shape. Cloud allowlists remain unchanged.

Review notes stay local and are deliberately excluded from delivery ZIP
manifests; Task0068 exports only the review status. No provider, model, task
start, file mutation, external app or account action is involved.

The retained Turkish dark-theme capture is
[owner-review-notes-tr-dark.png](../screenshots/owner-review-notes/owner-review-notes-tr-dark.png)
(2560×1760, SHA-256
`06036dd46550059378584cd899d0c48fc1ea6986ad2032b926b0166e944bb6c1`).

## Verification

- Core coverage proves bounds, canonical text, V15 replay, V16 recovery,
  clearing, exact hash scope and restart durability.
- Delivery coverage uses private canaries to prove notes never enter schema2
  manifests or delivery receipts.
- Desktop coverage proves Turkish/English note editing, exact command payloads,
  saved-note restoration and scroll reachability at the 800×560 minimum window.
- `cargo test -p patchfleet-desktop --all-features -- --test-threads=1`: 246
  passed.
- `cargo test --workspace --all-features -- --test-threads=1`: 541 passed; 6
  opt-in external-environment tests intentionally ignored.
- Strict workspace Clippy, formatting and diff checks pass.

## Limits

Notes are single-owner local context, not threaded comments, multi-reviewer
approval, a signature or automatic quality evidence. Windows result access
remains deferred to verified Windows-native no-reparse file operations.
