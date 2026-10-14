# Install and operate Patchfleet Desktop

Status: 0.4 native Rust desktop development distribution

Updated: 2026-10-14

## Application requirements

- macOS 11+; Windows 10+ with suitable OpenGL drivers; or Linux with native
  X11/Wayland and OpenGL runtime libraries. No WebView runtime is required.
- Provider CLIs installed separately. Their own Node/model/account requirements
  are independent of the Rust Patchfleet application.
- Native Windows/Linux installer and runtime checks remain CI/release evidence;
  the local development package has been built on macOS ARM64.

Use the platform installer: macOS DMG/application, Windows NSIS/MSI, or Linux
Deb/AppImage. Development artifacts have no public Developer ID notarization or
Windows publisher signature. Installers are built from source/CI; no public
release is claimed by this change.

For development, install Rust 1.99+ and platform compiler/window prerequisites.
Run `cargo run --locked -p patchfleet-desktop`. Build a production executable with
`cargo build --locked --release -p patchfleet-desktop`, then package it using
cargo-packager 0.11.8 and `Packager.toml`. Node is only legacy reference tooling.

For a native 0.4 upgrade, close the old application and replace its application
bundle through the reviewed installer. The bundle identifier and data directory
remain the same; existing Rust state remains compatible. Uninstalling must not
delete the canonical data directory.

## Local operation

Register a Git workspace in the desktop, select it when queueing work, and use
the provider cards to verify availability. Codex control requires a tested
stable app-server surface on 0.144.1 or newer. The sandbox remains
`workspace-write` with approval policy `never`. Claude and Gemini control are
not advertised.

Form teams from installed packs and templates. Background advancement does not
require a manual refresh. Owner questions and approval gates remain explicit.
Cancel a team before unregistering its workspace. Closing the desktop shuts
down owned provider processes; reopening marks lost ownership rather than
reattaching or resending uncertain work.

The platform application-data directory is:

| Platform | Default |
|---|---|
| macOS | `~/Library/Application Support/com.patchfleet.desktop` |
| Windows | `%APPDATA%\com.patchfleet.desktop` |
| Linux | `$XDG_DATA_HOME/com.patchfleet.desktop` or `~/.local/share/com.patchfleet.desktop` |

An explicitly set absolute `PATCHFLEET_DATA_DIR` selects an alternate directory.
One Rust host owns it. Unix modes are 0700/0600; Windows replaces inherited
grants with a protected current-owner+SYSTEM ACL. Private state files reject
symlink/reparse redirection. Do not run the old host against this directory.

Cloud pairing is optional. Pair through the desktop Cloud panel using a
short-lived code. Invalid pairing storage leaves local operation available and
reports recovery needed; close the app, back up the private file and repair or
remove it explicitly before re-pairing. Credentials never appear in IPC status.

## Migrating from 0.2

1. Stop the old Node host with its old `patchfleet stop` command. Close any
   development server using the same directory.
2. Back up the entire old directory while stopped. Keep it unchanged for rollback.
3. Copy `events.jsonl` into a new empty desktop data directory. Optionally copy
   `cloud.json` to retain pairing; it contains private credentials. Do not copy
   old `runtime.json`, `writer.lock` or derived projection files.
4. Start the Rust desktop with that destination selected. It validates complete
   records, recovers an incomplete final tail on the next append, rebuilds caches,
   finishes pending local state receipts and marks old provider ownership lost.
5. Verify work/workspaces/teams before creating new work. No provider side effect
   is replayed merely because the old request existed.

New V2 audit events require the Rust reader. Rollback restores the unchanged
pre-migration backup to the old host; never point the old reader at upgraded data.

## Offline Rust CLI

The optional native CLI uses the same core and data directory:

```sh
cargo run -p patchfleet-core --bin patchfleet -- --help
cargo run -p patchfleet-core --bin patchfleet -- --data-dir /absolute/test-data recover
```

`status`, `recover`, and `action ACTION < payload.json` require the desktop to be
closed because it owns the writer lock. `run` starts the headless owner until
Ctrl-C. `action` accepts the same validated action vocabulary as the desktop;
it cannot run arbitrary shell strings. Native hook ingress writes a sanitized
inbox rather than opening the canonical log.

## Gemini lifecycle ingress

The legacy source extension still uses its checked-in Node helper. For the
desktop, configure the owner-approved native hook command with Gemini's own
extension/settings consent. The desktop executable supports `--gemini-hook`,
reading bounded native hook JSON on stdin and storing only lifecycle metadata.
Use the same absolute data directory as the desktop. The executable path and
quoting are platform-specific; Patchfleet does not rewrite provider settings.

## Upgrade and uninstall

Close the application, back up its data, and install a reviewed compatible
replacement. Uninstall the application through the OS while keeping owner data
for recovery. Delete that directory only through an explicit owner action.
