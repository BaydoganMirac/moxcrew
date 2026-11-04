# Public preview release

Status: Published as GitHub pre-release `v0.12.0`

Updated: 2026-11-04

The owner authorized a GitHub pre-release for Moxcrew 0.12.0 so users can install
native packages without building from source. The public surface now leads to
the versioned release page and discloses the absent Apple notarization.

The first candidate CI run stopped before packaging on three targets. Review
first exposed deterministic platform/test-harness issues: Windows executable
discovery, Unix-only secure-output fixtures and parallel local-writer/timing
contention. A later no-fail-fast Windows run also exposed unresolved private
storage ACL/locking and platform-dependent desktop failures. Windows is therefore
withheld rather than represented as a usable installer.

The tag points directly to commit
`921db2132f407b4419c7c1c8d8f804ede3bacc84`. General CI run `41818496085`
passed on every matrix target. Native run `41818496580` passed on macOS Apple
Silicon, macOS Intel, Linux x64 and Linux ARM64; its experimental Windows job
failed the known native behavior gate and produced no public package.

The [published pre-release](https://github.com/BaydoganMirac/moxcrew/releases/tag/v0.12.0)
contains two DMGs, x64/ARM64 AppImage and Deb packages, and
`SHA256SUMS.txt`. Both DMGs passed `hdiutil verify`; extracted Mac executables
matched their advertised architectures and bundle version `0.12.0`; AppImages
matched their advertised ELF architectures; and both Deb files had valid Debian
archive structure. GitHub's uploaded asset digests matched the local SHA-256
manifest, and all seven anonymous download URLs returned HTTP 200.

The release remains unsigned and Apple-notarization, hands-on fresh-install /
upgrade evidence, live-provider quality evidence and Windows readiness remain
future production gates.
