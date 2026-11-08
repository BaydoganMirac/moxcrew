# Public preview release

Status: Published as GitHub pre-release `v0.13.1`

Updated: 2026-11-08

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
