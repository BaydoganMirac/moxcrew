# Project delivery packages: core

Updated: 2026-11-04

Status: Complete; focused checks and integrated independent review pass.

Plan: [Task0047](../plans/0047-project-delivery-packages.md).
Decision: [ADR0046](../decisions/0046-project-delivery-archives.md).
Integrated evidence: [delivery packages](delivery-packages.md).

`project_delivery_export` uses the existing native V12 owner requested/receipted
audit. Its payload is `{projectId, files, targetPath}`; each of 1–32 files is
exactly `{workItemId, path, expectedSha256}`. Nested shapes, paths, hashes and
duplicate work/path identities are checked before the request is accepted. The
Host resolves every selected managed job within the Project, reads pinned regular
files, and verifies the expected SHA-256. Limits are 25 MiB per file and 64 MiB
aggregate. All source bytes are checked before creating the destination.

The output is a standard UTF-8 STORE ZIP with CRC32, central directory and no
ZIP64/compression dependency. Numbered relative folders (`files/01/name.ext`)
prevent colliding names while retaining full sanitized Unicode basenames and
extensions without increasing the basename length. `manifest.json` schema2 records
Project ID and each work/path/archive-path/hash/byte identity, plus
`ownerReview.status` resolved from the exact Project/work/path/hash review
projection. No matching decision is represented as `unreviewed`; another hash's
decision is never inherited. The manifest adds no review note/time, absolute
paths, goals, prompts, account state or credentials. Explicitly selected source bytes
are exported unchanged; their content is not redacted. The command payload and
success receipt remain backward-compatible and do not gain review fields.

Single-file and package exports share the checked destination helper: absolute
owner-selected target, existing parent, outside the canonical data directory,
pinned no-follow access and atomic no-replace publication. Windows remains
explicitly unsupported until pinned no-reparse result operations are verified.
On Unix, an I/O failure publishes no partial target; a destination that appears
concurrently is preserved. See [Task0060](../plans/0060-atomic-owner-exports.md)
and [ADR0051](../decisions/0051-atomic-owner-export-publication.md).

Success returns `PROJECT_DELIVERY_EXPORTED`, Project ID, file count, archive byte
length (`bytes`), total source bytes (`sourceBytes`), archive SHA-256 and the safe
file provenance summary. No new persisted artifact or Cloud entity is created.
Exact duplicate commands return their durable receipt, including after restart.
A crash after writing the ZIP but before the outer receipt remains
`OWNER_COMMAND_OUTCOME_UNKNOWN`; the Host never repeats an uncertain export.

Six focused tests exercise real Host exports, Unicode/colliding names, 250–253-byte
basenames with retained `.pptx` extensions, binary
bytes, ZIP CRC/offsets/manifest, exact receipts/restart, crash prefixes, changed or
missing files, foreign/missing jobs, traversal, source/destination symlinks,
duplicate selections, count/size/aggregate limits, internal/existing targets and
no destination creation on source failures. A separate ignored fixture test
produces a retained real Host ZIP for an independent standard-reader check.

No provider process, inference, network call, external tool, new dependency or
Cloud change is needed. Native Windows execution and deliberate disk-full
injection were not performed; success is acknowledged only after staged bytes
are flushed, synced and published.
