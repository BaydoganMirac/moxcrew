# Review-aware delivery packages

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0068](../plans/0068-review-aware-delivery.md)

Delivery selection now shows the owner-review state of every exact artifact
version. Each visible row and the inspectable selected-files menu distinguish
**reviewed**, **changes needed** and **not reviewed**. The footer totals all
selected states and warns when the package contains a needs-changes or unreviewed
file. These warnings inform the owner but do not silently remove files or block
an intentional export.

The exported `manifest.json` is now schema2. Every file retains its Project,
work item, source path, archive path, byte length and SHA-256 and adds
`ownerReview.status`. The Host resolves this status from the exact current
Project/work/path/hash tuple while checking the actual exported bytes. A missing
matching decision is written as `unreviewed`; a decision for an older hash is
never inherited.

Review metadata appears only in the explicitly exported archive. The existing
delivery command and durable receipt shapes remain unchanged, no review time or
file content is added to the manifest, and no provider/model/account action is
involved. Review state remains human workflow evidence rather than factual,
semantic, visual or playback verification.

The retained Turkish dark-theme capture is
[review-aware-delivery-tr-dark.png](../screenshots/review-aware-delivery/review-aware-delivery-tr-dark.png)
(2560×1760, SHA-256
`377f8a816d272b87086b2377a3b3ed96f1552cf2e314a4d24644fb0ab008b722`).

## Verification

- Core delivery coverage proves reviewed, needs-changes and synthesized
  unreviewed states in schema2 while the receipt stays backward-compatible.
- Desktop coverage proves exact-hash isolation, Turkish/English copy, warnings,
  non-blocking explicit export and minimum-window reachability.
- `cargo test -p patchfleet-desktop --all-features -- --test-threads=1`: 246
  passed.
- `cargo test --workspace --all-features -- --test-threads=1`: 541 passed; 6
  opt-in external-environment tests intentionally ignored.
- Strict workspace Clippy, formatting and diff checks pass.

## Limits

The manifest reports the current owner-authored marker at export time. It is not
a signature, multi-reviewer approval, semantic quality score or rendered output
proof. Windows result access and package export remain deferred to verified
Windows-native no-reparse file operations.
