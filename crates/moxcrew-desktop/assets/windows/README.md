# Windows installer identity

These are the cargo-packager **0.11.8** NSIS and WiX templates, vendored under
its MIT license (included alongside them). Upstream:
https://github.com/crabnebula-dev/cargo-packager/tree/9e88b74ffb5531acf59c8bc371189a5e82b927d2/crates/packager/src/package

The current package uses Moxcrew for its install directory, registry identity,
Start Menu directory, executable and AppUserModel.ID. NSIS recognizes the exact
previous Patchfleet key and removes a recognized old install before copying the
new executable. A silent install with a legacy registration stops because it
cannot display the upgrade choice. WiX retains exact legacy lookups only for
shortcut cleanup and uses the previous MSI upgrade family GUID, so an old
WiX package is replaced when that format is used.

Legacy shortcut cleanup is restricted to Patchfleet.lnk in the owned desktop
and Start Menu locations. NSIS requires existing product registry metadata.
No appdata path is added and no canonical data directory is deleted. The Rust
Host migrates the separate previous Roaming AppData profile on first launch.

Reconcile this small delta when deliberately upgrading cargo-packager. Runtime
upgrade/uninstall validation on Windows remains a release gate; macOS cannot
execute NSIS or Windows Installer.

Original crate source SHA-256:

- `src/package/nsis/installer.nsi`: `3ff5fde09ae24dfc031ab91f939b290f394e2c6d3549f2c43c37414716a47b7a`
- `src/package/wix/main.wxs`: `393b9e4b821eb824864fab0637a3f7e77c1d2f50a7c8f9ab3e2d4d12437f2b67`
