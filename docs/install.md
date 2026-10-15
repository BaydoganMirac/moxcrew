# Install and operate Patchfleet Desktop

Status: 0.6.0 native Rust desktop development distribution

Updated: 2026-10-15

## Application requirements

- macOS 11+; Windows 10+ with suitable OpenGL drivers; or Linux with native
  X11/Wayland and OpenGL runtime libraries. No WebView runtime is required.
- Provider CLIs installed separately. Their own Node/model/account requirements
  are independent of the Rust Patchfleet application.
- Native Windows/Linux installer and runtime checks remain CI/release evidence;
  the local development package has been built on macOS ARM64.
- Linux folder selection needs a working XDG desktop portal with a desktop
  backend, or Zenity as the fallback. The Deb declares these runtime packages;
  AppImage users need them supplied by their distribution. GTK is not bundled.

Use the platform installer: macOS DMG/application, Windows NSIS/MSI, or Linux
Deb/AppImage. Development artifacts have no public Developer ID notarization or
Windows publisher signature. Installers are built from source/CI; no public
release is claimed by this change.

For development, install Rust 1.99+ and platform compiler/window prerequisites.
Run `cargo run --locked -p patchfleet-desktop`. Build a production executable with
`cargo build --locked --release -p patchfleet-desktop`, then package it using
cargo-packager 0.11.8 and `Packager.toml`. Node is only legacy reference tooling.

For a native 0.6 upgrade, close the old application and replace its application
bundle through the reviewed installer. The bundle identifier and data directory
remain the same; existing Rust state can be read by the new application. Back up
the stopped data directory before creating decisions: new V4 facts require
the 0.6 reader. Rolling back to 0.5 or earlier requires that pre-upgrade backup. Uninstalling must not
delete the canonical data directory.

### macOS opens an entirely white window

An already-running 0.3 WebView copy can be reactivated even when opening the
0.5 native application from Applications: both have the same bundle identifier.
This launch routing was reproduced with disposable data. The installed 0.5
production application draws correctly when the old process is closed.

1. Quit all running Patchfleet copies with Command-Q.
2. Eject the older Patchfleet disk image and close its installer window.
3. Open `Applications/Patchfleet.app`. The sidebar should show `Patchfleet 0.6.0`
   and the Türkçe/English controls.

If a Dock shortcut still opens the older disk-image copy, remove that shortcut
and add the application from Applications. No data or preferences need to be
deleted. See the [verified production window](screenshots/native-production-0.5-tr.png).

## Local operation

### Project decisions

Create a project and add messages in Memory (linked task/team sources also
appear there). Open Decisions, select the project and choose **New decision
draft**. Write the title/decision and cite one to eight current source versions.
Context, rationale and consequences are optional. Saving a draft does not
make it active. Review its exact evidence and choose **Accept decision** or
**Reject decision** explicitly. If a source changed before acceptance, create a
fresh draft citing current evidence; the stale draft can still be rejected.

To change an accepted ADR, create a new sourced draft, then select it as the
replacement on the accepted decision. Confirm the replacement action: the old
record becomes superseded and the new one accepted together. Original text,
source versions and history remain readable. Keyword/status filters stay within
the selected project. Local decisions require no account or subscription.
Vector retrieval, agent context tools, automatic extraction and Cloud
publication are not enabled by this version.

### Git folders and work

On Projects, choose **Choose folder** or **Add folder**. The system folder picker
opens; select the actual Git repository root, then confirm **Add folder** in
Patchfleet. The chosen path is read-only in the form. **Change folder** opens the
picker again; canceling leaves the draft intact. One-off work folders use the
same picker. A container holding several repositories is not itself a Git root:
choose the child containing `.git`, including a linked worktree's `.git` marker.

Patchfleet checks the selected folder before displaying the confirmation and
checks current access again when registering a new command. A missing folder,
non-repository, unsafe root, denied access and read-only folder have separate
localized explanations. These checks inspect directory metadata, not source
contents. Registration does not grant provider processes unrestricted access.

For denied access, reselect the folder and accept the OS prompt when offered.
On macOS, the form can open **System Settings > Privacy & Security > Files and
Folders**. The App supplies purpose descriptions for Documents, Desktop,
Downloads, removable and network volumes. A read-only folder instead points to
**Finder > Get Info > Sharing & Permissions** or a writable disk. Windows
guidance points to folder **Properties > Security**; Linux guidance points to
read/write/traverse permissions in the file manager. Patchfleet does not change
ACLs, enable Full Disk Access or silently modify folder permissions. OS consent
and filesystem access remain under the owner's control.

Success, rejection and failure feedback have distinct styles, a dismiss action
and relevant recovery. Internal codes are in **Technical details**. Retry keeps
the original command identity; choosing a different folder starts a new action.
An exact canonical retry of an already-recorded registration returns its
original receipt even if that folder has since lost access or been deleted.

Select an added folder when queueing work, and use
the provider cards to verify availability. Codex control requires a tested
stable app-server surface on 0.144.1 or newer. The sandbox remains
`workspace-write` with approval policy `never`. Claude and Gemini control are
not advertised.

Create a stable Project and explicitly link one or more registered worktrees
from its card after adding them.
Existing unlinked worktrees stay usable. Project Memory stores messages and owner
sources for new bound tasks/teams; it does not silently import historical work or
provider transcripts. Select a project to read its bounded source page, filter by
keyword, enter a message or revise a directly entered message. Captured task/team
sources preserve their original text and project. Unlinking/unregistering a worktree
does not erase history. Source records alone are context; authoritative ADRs
are managed in Decisions. Semantic/vector search, agent memory tools and
automatic extraction remain unavailable.

Choose Türkçe or English in the sidebar. Turkish is the default; the selected
interface language is saved locally in protected `desktop-preferences.json`.
Source and task text are never automatically translated. A damaged preference
falls back to Turkish and shows a safe message; explicitly selecting a language
replaces only this display configuration, preserving canonical work and memory.

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
