# Public preview release

Status: 0.13.3 public preview published for Windows, macOS and Linux

Updated: 2026-11-20

## Published 0.13.3 preview

The [0.13.3 release](https://github.com/BaydoganMirac/moxcrew/releases/tag/v0.13.3)
ships unsigned Windows x64 EXE and MSI installers, two macOS DMGs, and AppImage
and Deb packages for Linux x64 and ARM64. The tag points to
`0731e341b15b2a0fabd3cb7c1d9a71a655790166`.

All five platform jobs and the publication job passed in the
[tagged workflow](https://github.com/BaydoganMirac/moxcrew/actions/runs/44771741853).
The Windows job checked both installer payloads against the built executable and
tested upgrade and uninstall with existing profiles. After publication, all
eight installers downloaded anonymously and their SHA-256 digests matched the
[published manifest](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.3/SHA256SUMS.txt).

## Windows file delivery verification

The checked-in [Windows delivery contract](../plans/0111-windows-file-delivery.md)
covers project output listing, text preview, individual export and ZIP delivery.
On Windows a separate Host CLI process performed each operation on a disposable
project containing a UTF-8 Turkish report. Python's standard ZIP reader reopened
the delivered file, checked CRCs and compared its bytes and SHA-256 to the
original. Repeating each export after reopening the Host returned the first receipt.

The six focused Windows storage and account suites passed 57 tests. The Host
recovered an incomplete final event, preserved the original writer lock and
restored the prior project's files and receipts. A separate upgrade fixture
opened an existing profile in place and exported the original job's output.
The local JavaScript reference suite passed 153 tests.

The tagged native workflow verifies two Windows installers, two macOS DMGs and
four Linux packages. It records SHA-256 for all eight before publishing. The
Windows preview saves exports on local volumes and ships without a publisher
signature.

## Previously published 0.13.1 preview

The owner authorized a GitHub pre-release for Moxcrew 0.13.1 so users can install
the adaptive-recovery build without compiling it. The public surface leads to
the versioned release page and discloses the absent Apple notarization.

The tag points directly to commit
`029195074f13577c1463ada7eebcad9674ad325c`. General CI run `42504299936`
passed on every matrix target. Pre-tag native run `42504300101` and tagged
release run `42507226401` passed on macOS Apple Silicon, macOS Intel, Linux x64
and Linux ARM64. The experimental Windows job failed the known native behavior
gate in both native runs, remained non-blocking and produced no public package.

The [published pre-release](https://github.com/BaydoganMirac/moxcrew/releases/tag/v0.13.1)
contains two DMGs, x64/ARM64 AppImage and Deb packages, and
`SHA256SUMS.txt`. The tagged workflow verified package names and the complete
six-installer set before publishing. GitHub's uploaded asset digests match the
published SHA-256 manifest, and all seven anonymous download URLs return HTTP
200.

The release remains unsigned. Apple notarization, hands-on fresh-install /
upgrade / rollback evidence, live-provider quality evidence and Windows
readiness remain production gates; this publication does not claim those checks.
