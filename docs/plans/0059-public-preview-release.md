# Task0059: Public preview release

Status: Owner-approved; implementation in progress

Updated: 2026-11-04

Authorization: the owner explicitly requested a GitHub release so prospective
users can install Moxcrew directly instead of being sent first to source-build
instructions.

## Goal

Publish Moxcrew 0.12.0 as an honest GitHub Public Preview with downloadable
native installers for the targets already built and verified by the repository's
native CI. Make download the primary README path while keeping source build and
recovery instructions available.

## Release shape

- Tag and title: `v0.12.0`, “Moxcrew 0.12.0 — Public Preview”.
- GitHub classification: pre-release, not a signed production release.
- Assets: macOS Apple Silicon/Intel DMGs, Linux x64/ARM64 AppImage/DEB, plus
  SHA-256 checksums. Windows is excluded until its native private-storage and
  behavior gates pass.
- Source: successful native CI artifacts for the exact reviewed `main` commit.
- Copy: direct platform choices, compatibility/data-backup warning and explicit
  disclosure that macOS notarization and Windows publisher signing are absent.

## Boundaries and acceptance

Do not invent signing, notarization, antivirus reputation, live-provider quality
or native runtime evidence that has not been established. Do not publish npm,
Cloud or website changes. Preserve package/bundle/data compatibility identifiers.

Before publication, general CI and every advertised macOS/Linux native job for
the selected commit must pass. The Windows experimental job must still run and
remain visible, but it is not a release asset gate until its known failures are
resolved. Inspect every downloaded archive, verify expected installer types and
hash all public assets. After publication, update README/install/release/state
documentation, GitHub About homepage and verify that the tag, assets and links
are publicly readable.

## Implementation note

The first candidate matrix exposed cross-platform test-runner issues before any
package was published: Windows optional-tool discovery did not account for
`.exe`, four Unix-only secure-output fixtures ran on Windows, and parallel tests
could contend for the single local writer or fixture deadlines. The release
candidate fixes those boundaries and runs native behavior tests serially before
building installers.

A later no-fail-fast Windows run exposed broader private-storage ACL/lock and
platform-dependent desktop test failures. The release scope therefore narrows to
the four passing macOS/Linux targets instead of presenting an unverified Windows
installer as usable.
