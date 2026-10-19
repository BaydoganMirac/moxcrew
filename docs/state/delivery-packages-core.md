# Project delivery packages: core

Updated: 2026-10-19

Status: Implemented; focused native checks pass, integrated review pending.

Plan: [Task0047](../plans/0047-project-delivery-packages.md).
Decision: [ADR0046](../decisions/0046-project-delivery-archives.md).

`project_delivery_export` uses the existing native V12 owner requested/receipted
audit. Its payload is `{projectId, files, targetPath}`; each of 1–32 files is
exactly `{workItemId, path, expectedSha256}`. Nested shapes, paths, hashes and
duplicate work/path identities are checked before the request is accepted. The
Host resolves every selected managed job within the Project, reads pinned regular
files, and verifies the expected SHA-256. Limits are 25 MiB per file and 64 MiB
aggregate. All source bytes are checked before creating the destination.

The output is a standard UTF-8 STORE ZIP with CRC32, central directory and no
ZIP64/compression dependency. Numbered relative aliases prevent colliding names
while retaining readable Unicode filenames. `manifest.json` schema1 records
Project ID and each work/path/archive-path/hash/byte identity. The package includes
no absolute paths, goals, prompts, account state or credentials.

Single-file and package exports share the checked destination helper: absolute
owner-selected target, existing parent, outside the canonical data directory,
pinned no-follow creation and no overwrite. Windows remains explicitly
unsupported until pinned no-reparse result operations are verified. An I/O
failure can leave a partial target. Neither export removes the public path on
failure, since another process may have replaced it.

Success returns `PROJECT_DELIVERY_EXPORTED`, Project ID, file count, archive byte
length (`bytes`), total source bytes (`sourceBytes`), archive SHA-256 and the safe
file provenance summary. No new persisted artifact or Cloud entity is created.
Exact duplicate commands return their durable receipt, including after restart.
A crash after writing the ZIP but before the outer receipt remains
`OWNER_COMMAND_OUTCOME_UNKNOWN`; the Host never repeats an uncertain export.

Five focused tests exercise real Host exports, Unicode/colliding names, binary
bytes, ZIP CRC/offsets/manifest, exact receipts/restart, crash prefixes, changed or
missing files, foreign/missing jobs, traversal, source/destination symlinks,
duplicate selections, count/size/aggregate limits, internal/existing targets and
no destination creation on source failures. A separate ignored fixture test
produces a retained real Host ZIP for an independent standard-reader check.

No provider process, inference, network call, external tool, new dependency or
Cloud change is needed. Native Windows/Linux execution and deliberate disk-full
injection were not performed; success is acknowledged only after flush/sync.
