# Task0095: Repeatable public preview release

Status: Complete; locally verified, next tag not published

Updated: 2026-11-05

Authorization: the owner asked to send the accumulated product work to GitHub
and make installation lead to a direct Release download instead of source-build
instructions. The existing `v0.12.0` preview is public and must remain unchanged;
this card makes the next explicitly pushed version tag reproducible.

## Problem

The native matrix builds development installers and retains compressed workflow
artifacts, but publishing the six user-facing macOS/Linux installers and their
checksum manifest is still a manual operation. A new tag can therefore drift
from README asset names or omit a platform even when the package jobs passed.

## Scope

1. Accept only exact `v0.MINOR.PATCH` preview tags whose version matches
   both the Rust workspace and `Packager.toml`.
2. Reuse the existing native matrix and upload only two macOS DMGs plus x64 and
   ARM64 Linux AppImage/Deb assets under stable marketing-facing names.
3. Withhold every Windows output while its native gates remain experimental.
4. Aggregate exactly six installers, generate and verify `SHA256SUMS.txt`, then
   create a GitHub pre-release with bounded public-preview notes.
5. Preserve pull-request/main CI behavior, use only the repository token and
   add no signing, notarization, Cloud, provider or deployment authority.

## Acceptance

- Non-tag pushes cannot stage public assets or create a release.
- A stable-major, invalid or mismatched tag fails before packaging.
- A missing or duplicate installer fails before publication.
- Release creation waits for every required native matrix job and publishes no
  Windows artifact.
- Workflow YAML, shell branches, formatting and repository checks pass.
- Documentation states that pushing/tagging/publication remain explicit owner
  actions; this local change does not publish or replace a release.

## Outcome

The existing native matrix now accepts only an exact version-matched `v0.x.y`
tag, stages the six advertised macOS/Linux installers under stable public names,
verifies their count, creates a basename-only SHA-256 manifest and publishes one
GitHub pre-release after the required matrix completes. Global workflow access
remains read-only; only the tag-gated release job receives `contents: write`.

The current `v0.12.0` release was inspected but not changed. A real feature-off
ARM64 release binary and App were built from the current product, and a separate
version-named local DMG passed signature, bundle, permission-plist and image
integrity checks. No tag, push, GitHub release, signing, notarization or Windows
publication occurred.
