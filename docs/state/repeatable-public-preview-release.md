# Repeatable public preview release

Status: Complete; locally verified, next tag not published

Updated: 2026-11-05

Plan: [Task0095](../plans/0095-repeatable-public-preview-release.md)

## Outcome

The native desktop workflow now turns a reviewed exact `v0.MINOR.PATCH` tag into
the same direct-download shape as the current public preview. The tag must match
both the Rust workspace and `Packager.toml`; ordinary pull-request, `main` and
manual workflow runs cannot stage public assets or create a release.

Each required macOS/Linux native job still runs formatting, behavior tests,
strict Clippy, the feature-off release build, browser-engine dependency rejection,
packaging and platform checks. Tag runs additionally rename only the two DMGs,
two AppImages and two Deb packages to stable Moxcrew asset names. Windows remains
visible and experimental in the matrix but cannot enter the public asset set.

The release job has the workflow's only `contents: write` permission. It waits
for the native matrix, downloads exactly six installers, rejects missing or
duplicate files, writes a basename-only `SHA256SUMS.txt`, verifies it and calls
`gh release create --verify-tag --fail-on-no-commits --prerelease`. It uses only
the scoped repository token and adds no signing, notarization or deployment
secret.

The already published `v0.12.0` pre-release was read back from GitHub on
2026-11-05. It remains non-draft, contains the expected six installers plus
`SHA256SUMS.txt`, and exposes GitHub SHA-256 digests for every asset. No release
asset, tag or remote branch was changed.

## Verification

- Ruby/Psych parsed the workflow and `bash -n` accepted all 16 shell blocks.
- The exact current `v0.12.0` tag/version contract passed locally. Stable-major,
  mismatched, incomplete and prerelease-suffixed samples failed closed.
- The release-authority regression check now permits only the exact tag-scoped
  native publisher while still rejecting npm publication and OIDC write access.
  The complete legacy/reference suite passed under the repository-required Node
  24.22.1: 153 tests, zero failures.
- `cargo build --locked --release -p patchfleet-desktop` passed with Rust 1.99.1.
- Pinned cargo-packager 0.11.8 produced the real ARM64 `Moxcrew.app`; it has
  bundle identifier `com.patchfleet.desktop`, version `0.12.0`, the required
  folder-use descriptions and a valid strict ad-hoc signature. Its Mach-O binary
  is 18,382,432 bytes, SHA-256
  `de1255c082e5b9eaf6315d01482247bb6270e68dd3667edc4754fc9f36eaff9c`.
- An existing owner-mounted `Moxcrew` volume prevented cargo-packager's default
  create-dmg script from unmounting its chosen volume name. Those mounts were
  preserved. The documented distinct-volume fallback produced
  `dist/release-automation-aarch64/Moxcrew_0.12.0_release-automation_aarch64.dmg`,
  9,358,018 bytes, SHA-256
  `d9318f6f00dc5bf067361023ebdba707b1e546e0834c4e2f60c52fb6a315d8fe`;
  `hdiutil verify` reported a valid image.
- Workflow/docs diff checks and formatting checks pass. The immediately prior
  product commit passed the sequential all-target/all-feature workspace suite
  with 601 tests, 16 explicit fixture/manual ignores and zero failures; this
  task changes no Rust runtime behavior.

## Limits

GitHub-hosted execution is deliberately unclaimed until the reviewed commits
and a new version tag are explicitly pushed. The local fallback DMG verifies the
current App and image contract but is not a public asset and does not replace
clean-runner matrix evidence. Apple notarization, publisher signing, hands-on
fresh install/upgrade, live-provider quality and Windows remain separate gates.
