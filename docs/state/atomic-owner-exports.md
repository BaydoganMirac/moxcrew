# Atomic owner exports

Updated: 2026-11-04

Status: Complete; macOS runtime and full workspace verification pass.

Plan: [Task0060](../plans/0060-atomic-owner-exports.md).
Decision: [ADR0051](../decisions/0051-atomic-owner-export-publication.md).

Single-file output export and Project delivery ZIP export now share one staged
destination. Moxcrew opens a random `0600` sibling file through the pinned owner
destination directory, writes and syncs the complete bytes, then publishes the
requested name atomically without replacing an existing file. Linux uses
`renameat2(RENAME_NOREPLACE)`; macOS uses `renameatx_np(RENAME_EXCL)`. Other Unix
targets use a no-replace hard-link fallback. No new dependency was added.

The requested public path is never the write-in-progress file. A failed source
read, bounded copy, ZIP write, flush or sync drops the private staging file. If
another process creates the target before publication, its bytes remain intact
and the existing `PROJECT_RESULT_EXPORT_EXISTS` result is returned. Cleanup only
addresses Moxcrew's random staging name. Existing absolute-path, internal-data,
link, scope, size, receipt/replay and Windows fail-closed checks are unchanged.

Verification includes focused tests proving that:

- the requested path is absent before commit and complete after commit;
- a source fails after seven staged bytes without publishing a partial target;
- an owner replacement created during that failed copy survives unchanged;
- a commit-time collision preserves the competing file; and
- no `.moxcrew-export-*` staging file remains after those outcomes.

The complete workspace/all-target suite passes **524 tests, zero failures and
six ignored opt-in tests**. Strict workspace/all-target/all-feature Clippy with
warnings denied, workspace formatting and `git diff --check` pass. The checks
also retain real Host single-file export, delivery ZIP, exact receipt/restart,
traversal, symlink, hash, collision and crash-prefix coverage.

No provider inference, account action, network request, event/Cloud schema,
signing or publication was added. Windows result access remains unsupported by
owner direction until it can be tested on Windows hardware. The current host is
macOS; native Linux runtime verification and deliberate disk-full injection are
not claimed by this local evidence.
