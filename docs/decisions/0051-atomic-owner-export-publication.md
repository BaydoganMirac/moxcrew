# ADR0051: Atomic no-replace owner export publication

Status: Accepted for owner-authorized Task0060

Date: 2026-11-04

## Decision

Write owner exports to a random private sibling staging file and publish it with
the platform's atomic no-replace rename operation only after the file is complete
and synced. Linux uses `renameat2(RENAME_NOREPLACE)` and macOS uses
`renameatx_np(RENAME_EXCL)` against the already pinned destination directory.
Other Unix targets retain a no-replace hard-link fallback. The public target is
never used as the write-in-progress file.

The staging name is not an artifact identity and never enters an event, receipt,
snapshot or Cloud payload. Failure cleanup may remove only that random staging
name; it never removes the requested public path. A commit-time collision keeps
the competing file and returns the existing export-collision reason.

## Rationale

Creating the final path before copying prevented overwrite races, but a disk or
source failure could leave a plausible-looking partial file. A same-directory
staged rename preserves no-overwrite behavior while making visibility atomic.
The standard library has no portable no-replace rename, so the existing `libc`
dependency supplies the two release-platform syscalls.

## Preserved boundaries

The current single-writer command and durable receipt contracts remain
unchanged. An export crash after publication but before its outer receipt is
still an uncertain side effect and is never automatically repeated. Windows
result access remains unsupported pending native implementation and testing.
