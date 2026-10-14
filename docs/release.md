# Desktop release

The 0.3 source package builds a Rust/Tauri desktop application. npm publishing
is retired; the old publish workflow was removed and package.json is private.
Publishing installers, pushing a branch, signing and deploying Cloud remain
explicit owner release actions.

## Verify a reviewed version

1. Run frontend tests/build, Rust workspace tests, Clippy and formatting checks.
2. Run `.github/workflows/desktop.yml` on the reviewed commit. Inspect native
   macOS ARM64/Intel, Windows x64 and Linux x64/ARM64 checks and installers.
3. Exercise startup, a fresh data directory, workspace/work/team flows, provider
   loss/cancel, normal exit and offline-copy migration on each advertised OS.
4. Verify the sanitized Cloud boundary using test fixtures and an approved
   staging pairing. Test signing/installer behavior independently of model access.
5. Preserve installer checksums and a pre-migration data backup. Do not claim an
   unrun platform or an ad hoc signature as a verified signed public release.

## Artifacts and signing

macOS produces application/DMG, Windows NSIS/MSI, and Linux Deb/AppImage outputs.
CI uploads development artifacts inside tar archives so executable permissions
survive download. The workflow does not publish a GitHub release or use secrets.

macOS public distribution needs Developer ID signing and notarization; Windows
publisher signing needs its own owner credentials. Keep those secrets out of
source and local model context. No signing credential is configured here.

## Rollback

Close the application and restore a compatible version plus its verified
backup. New local V2 audit events cannot be read by the Node 0.2 host; restoring
the old reader also restores the unchanged pre-migration directory. Never run
two canonical writers against one directory.

See [installation](install.md), [ADR 0023](decisions/0023-rust-tauri-desktop-runtime.md)
and [current evidence](state/rust-desktop.md).
