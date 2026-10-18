# Windows installer identity

These are the cargo-packager **0.11.8** NSIS and WiX templates, vendored under
its MIT license (included alongside them). Upstream:
https://github.com/crabnebula-dev/cargo-packager/tree/9e88b74ffb5531acf59c8bc371189a5e82b927d2/crates/packager/src/package

The intentional delta separates Moxcrew display labels from the existing
Patchfleet install directories, registry identities and Start Menu directory.
NSIS recognizes both old and new WiX display-name/publisher pairs. The same
uninstall registry key remains authoritative; shortcut AppUserModel.ID and
main executable identities remain configuration inputs.

Legacy shortcut cleanup is restricted to Patchfleet.lnk in the owned desktop
and Start Menu locations. NSIS requires existing product registry metadata.
WiX uses the same stable component registry keys as the previous installation.
No appdata path is added and no canonical data directory is deleted.

Reconcile this small delta when deliberately upgrading cargo-packager. Runtime
upgrade/uninstall validation on Windows remains a release gate; macOS cannot
execute NSIS or Windows Installer.

Original crate source SHA-256:

- `src/package/nsis/installer.nsi`: `3ff5fde09ae24dfc031ab91f939b290f394e2c6d3549f2c43c37414716a47b7a`
- `src/package/wix/main.wxs`: `393b9e4b821eb824864fab0637a3f7e77c1d2f50a7c8f9ab3e2d4d12437f2b67`
