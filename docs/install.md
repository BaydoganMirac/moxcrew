# Install and operate Moxcrew Desktop

Status: 0.13.2 native Rust desktop Public Preview

Updated: 2026-11-19

## Download

Download the installer for your platform directly:

- Windows x64: [Setup EXE](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_windows_x64-setup.exe)
  or [MSI](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_windows_x64.msi).
- macOS: [Apple Silicon DMG](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_macOS_Apple-Silicon.dmg)
  or [Intel DMG](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_macOS_Intel.dmg);
- Linux x64: [AppImage](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_linux_x86_64.AppImage)
  or [Deb](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_linux_x86_64.deb);
- Linux ARM64: [AppImage](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_linux_aarch64.AppImage)
  or [Deb](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/Moxcrew_0.13.2_linux_aarch64.deb).

No Rust, Node or source checkout is needed. Compare the installer's SHA-256
digest with the published [SHA256SUMS.txt](https://github.com/BaydoganMirac/moxcrew/releases/download/v0.13.2/SHA256SUMS.txt)
before opening it. Provider CLIs are separate: install and authenticate Codex,
Claude Code or Gemini CLI according to the provider you intend to use.

The Public Preview is not notarized with an Apple Developer ID. On macOS, use
Finder's **Open** action or the explicit **Open Anyway** control in Privacy &
Security after verifying the checksum. Moxcrew does not require a shell command
that disables an OS security feature.

On Windows, use the setup EXE for a per-user installation or the MSI for a
machine-wide installation. Both contain the same desktop executable. The
preview has no publisher signature. In PowerShell,
`Get-FileHash -Algorithm SHA256 .\Moxcrew_0.13.2_windows_x64-setup.exe`
prints the digest for the setup download. Substitute the MSI filename if needed.

When saving a project file or ZIP on Windows, choose a folder on a local
drive. You can copy the finished file to a network share afterward.

## Application requirements

- Windows x64, macOS 11+, or Linux with native X11/Wayland and OpenGL runtime libraries. No
  WebView runtime is required.
- Provider CLIs installed separately. Their own Node/model/account requirements
  are independent of the Rust Moxcrew application.
- The public release matrix builds packages on their native platforms.
  Publisher signing and hands-on upgrade coverage remain production gates.
- Linux folder selection needs a working XDG desktop portal with a desktop
  backend, or Zenity as the fallback. The Deb declares these runtime packages;
  AppImage users need them supplied by their distribution. GTK is not bundled.

The release workflow builds all eight installers from the exact tagged source
and publishes a checksum manifest alongside them.

For development, install Rust 1.99+ and platform compiler/window prerequisites.
Run `cargo run --locked -p moxcrew-desktop`. Build a production executable with
`cargo build --locked --release -p moxcrew-desktop`, then package it using
cargo-packager 0.11.8 and `Packager.toml`. Node is only legacy reference tooling.

New native installs use the `moxcrew-desktop` executable and
`com.moxcrew.desktop` bundle identifier. The Windows per-user EXE defaults to
`%LOCALAPPDATA%\Moxcrew`. The MSI installs under 64-bit Program Files.
Both use `%APPDATA%\Moxcrew` for a new Windows profile.
Close the old application before installing the new one.

For an upgrade to the Moxcrew identity, close the old application and back up
its stopped data profile before using the reviewed installer. The bundle
identifier changes. Moxcrew opens a recognized existing profile in its original
location so earlier jobs can still find their files. If both profile locations
exist, select the intended profile explicitly with `MOXCREW_DATA_DIR`.
On Debian, remove the older `patchfleet-desktop` package
after backing up its data, then install `moxcrew-desktop`; the new package name
is a separate package identity. Bound agent launches write V6
context facts requiring the 0.10 reader; rollback after V6 writes requires
that unchanged pre-upgrade backup. Extraction commands add V7 facts requiring
a 0.11 reader; older readers need the stopped pre-upgrade backup. Create job,
export and work retry commands now add V8 facts. Pre-Task0040 builds, including
older 0.12.0 development binaries, cannot read them; rollback restores the full
unchanged pre-upgrade copy. Task0041 adds V9 audit facts for conversation sends
and motion preferences; pre-Task0041 builds also reject these facts. The same
complete stopped-backup rollback rule applies. Task0052 adds V14 Project profile,
skill, trigger, alternative and evidence facts. Preserve the complete stopped
pre-upgrade copy before first use; builds without V14 support cannot read those
facts. Task0103 originally wrote V17 adaptive checkpoint facts; builds before
0.13 reject those facts. Preserve the complete stopped pre-upgrade copy before
the first adaptive mission and restore that copy for rollback. Never edit log
lines to downgrade. New strict adaptive checkpoints write V18 while retaining
V17 replay; take the same stopped backup before the first V18 write because
older builds reject it. Uninstalling must not delete the canonical data
directory. Adaptive terminal-failure recovery adds V19 reviewer allocations and
V20 recovery checkpoints. Preserve the stopped pre-upgrade copy before the first
recovery run; older builds reject those facts and event versions must never be
rewritten for rollback.

Version 0.13.2 records a direct Mox chat answer as a V21 event. Back up the
stopped profile before the first chat in this release. To return to 0.13.1,
restore that pre-upgrade profile so its reader can open the log.

### Historical macOS 0.5 white-window issue

In older 0.3/0.5 builds, an already-running WebView copy could be reactivated
because both used the former bundle identifier. This historical launch routing
was reproduced with disposable data. Current Moxcrew packages use the new
identifier specified above.

1. Quit all running Moxcrew / older Patchfleet copies with Command-Q.
2. Eject the older Patchfleet disk image and close its installer window.
3. Open `Applications/Moxcrew.app`. The sidebar should show `0.13.2`
   and the Türkçe/English controls.

If a Dock shortcut still opens the older disk-image copy, remove that shortcut
and add the application from Applications. No data or preferences need to be
deleted. See the [verified production window](screenshots/native-production-0.5-tr.png).

## Local operation

### Create a general-purpose task

The application opens the Project conversation in **Chat / Sohbet**. Create or
select a Project and describe your goal in the composer. Explicit goal terms
suggest a task type; select one manually when the goal is ambiguous. The full
catalog and inspectable instructions remain available. Select **Prepare task / Görevi hazırla**,
then explicitly start Codex when its supported provider is available.
No Git folder, Moxcrew account or Cloud subscription is required. Provider
accounts and usage costs remain independently managed. These project-bound jobs
require Codex 0.155.1+ and its scoped memory connection.

Chat, Projects and Tasks are the primary navigation. **More / Diğer** opens
teams, memory, decisions, agent packs, provider setup and Cloud. When Codex is
unavailable, the queued task shows provider setup next to its start action.
**Connections / Bağlantılar** separates CLI installation, saved sign-in and
supported task control. The check runs only on explicit opening/refresh and
does not sign in, install software or make a model call. Codex and Claude saved
credential checks do not prove entitlement/quota; Gemini sign-in remains unknown.
Only Codex currently has supported task control. Provider setup guides open by
deliberate action. Saved reduced motion disables Mox's active-work animation.
After installing or updating a provider CLI, restart Moxcrew, then check
connections again. The running controller retains the program paths resolved
at launch; a fresh GUI process may also be needed to pick up changed PATH.

Conversation history contains actual owner requests and work status. Open a
job's file panel to inspect its real outputs. Follow up on a completed, failed
or interrupted job in the same Project; optionally select up to eight files
(25 MiB total). The new task receives exact copies and a SHA-256 manifest in
inputs/, preserving the originals. A follow-up still requires explicit start.

The forty recipes guide the agent; they do not install missing renderers, model
access or tools. Review any unmet requirements in the output. Task completion
and file presence do not verify content quality. Read the job's actual output
list, preview supported UTF-8 Markdown/text/CSV up to 256 KiB, or export a file
up to 100 MiB using the system save dialog. Export refuses existing destinations.
To deliver several files together, select the outputs and save a ZIP. Its
manifest identifies each selected file and its SHA-256 digest. Moxcrew publishes
completed exports atomically and preserves a destination that appears during a save.
Canceling the dialog changes nothing. Retries use a fresh job directory and
retain the original Project and earlier outputs. Unsent text is session-only.

### Coding task with Quick start

Projects offers **Quick start / Hızlı Başlangıç**, with Mox
showing the next step. You can leave the guide and reopen it from Projects;
existing populated installations keep their normal view.

1. Create a project, or explicitly choose an existing one.
2. Select an existing Git repository with the system folder picker, confirm
   registration, then choose **Link folder**. Selecting or registering a folder
   alone does not link it. If the project has several folders, choose the one
   to use for this walkthrough.
3. Use **Refresh providers** or **Open providers** to check Codex compatibility.
   This check does not verify provider sign-in. Project-bound memory requires
   Codex 0.155.1 or later; provider accounts remain independently managed.
4. Choose **Write first task**, enter its name and bounded instruction, and
   select **Add to queue**. Review the queued task, then deliberately choose
   **Start Codex**. Queueing does not start an agent. The guide confirms a first
   start only after its applied receipt and corresponding project-bound run.

An existing task draft keeps its text and folder when opened through the guide.
Saved project/folder/start progress is reconstructed after restart; unsent task
text is session-only. On a failed action, follow the feedback and retry
deliberately. Moxcrew account sign-in and Cloud pairing are optional throughout.

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
Cloud semantic retrieval is available through explicit search. Agent context
tools are available for owned Codex runs; optional automatic drafts are described below.

### Optional automatic drafts

In Memory, select a project and read the **Automatic drafts** disclosure.
Check consent and enable it to send subsequent owner sources to the installed
Codex provider together with relevant accepted project decisions, including
older ADRs. The future-only rule applies to source queueing. This can use the provider account's quota; Local storage does
not mean offline model inference. Codex0.155.1+ is required; no API-key field,
Cloud account or Moxcrew subscription is needed.

Old sources are not automatically sent. Open a selected source and deliberately
queue its current version. Activity shows counts and only the latest10 queued
jobs; an older job remains accessible on its current source card. Failed/interrupted/cancelled jobs require manual retry, up to three
attempts. Disabling or editing the source cancels stale work. A question may
produce no ADR; exact existing text is skipped. In Decisions, inspect the
Codex-generated label, exact evidence and any potential relation before
accepting/rejecting through the existing controls. A draft never becomes an
agent rule automatically. See [extraction protocol](extraction-protocol.md).

### Agent access to project memory

Link the task’s registered folder to its project before queuing the task.
Owned Codex work and team roles receive three read-only memory tools automatically.
Only current accepted Local ADRs are readable, across all linked worktrees.
The task card shows its starting/current decision revision and whether the
current manifest was read. Changed decisions require the agent to refresh
project_context; drafts are never active. Local needs no account or subscription.
Memory attachment requires Codex 0.155.1 or later. An unsupported or failed
connection prevents that bound task from starting; fix the connection and start
the still-queued task again. Failed team starts pause automatic dispatch; retry
that task explicitly from Work.
Historical unbound tasks retain their previous behavior. There is no global MCP
setup or separate database process. See [agent protocol](agent-memory-protocol.md).

### Optional Cloud account

Open **Sign in · optional** in the sidebar or the Cloud section. In connection
settings, enter the root HTTPS origin of your compatible Cloud service and a
name for this installation. There is no assumed live default address yet.
Choose **Sign in to Cloud**, complete the existing browser login, compare the
displayed code and approve the installation. Keep Moxcrew open while waiting;
you can reopen/copy the browser link or cancel. No password is entered in the
native application. Local operation continues while sign-in is pending.

The account card shows the verified account and fresh subscription state, with
explicit refresh and a same-service billing link. An account without an eligible
subscription still signs in; Cloud memory enforces its own paid access. Signing
in does not publish Local ADRs or create/link a shared project automatically.
**Sign out** clears this installation's local credential and attempts remote
revocation. If offline, the notice explains that remote revocation was not
confirmed. Local projects and downloaded decisions remain. See
[account state](state/desktop-account.md) for verification and release limits.

### Optional shared decisions

This requires a deployed compatible Cloud memory service and a verified paid
subscription. In Cloud, select the Local project and explicitly create a shared
project on the publishing installation. In Decisions, open an accepted ADR,
preview the published fields, confirm the upload checkbox and publish it. Queued
means saved locally; only the server acknowledgement confirms publication.
Source text and provenance events remain local; the upload includes opaque
source ID/revision references only. Review the ADR text itself before sharing.

Pair a second installation to the same personal workspace, select its Local
project and explicitly link the shared project ID as a reader. The Cloud view
lists downloaded decisions and opens exact immutable versions. These copies
do not accept or overwrite Local decisions. Offline/access/subscription errors
preserve downloaded records and show their last verification time. The publisher
can disable sharing; this stops new reads/uploads, without erasing existing
offline copies. Publisher transfer and organization invitations are not included.

### Git folders and work

On Projects, choose **Choose folder** or **Add folder**. The system folder picker
opens; select the actual Git repository root, then confirm **Add folder** in
Moxcrew. The chosen path is read-only in the form. **Change folder** opens the
picker again; canceling leaves the draft intact. One-off work folders use the
same picker. A container holding several repositories is not itself a Git root:
choose the child containing `.git`, including a linked worktree's `.git` marker.

Moxcrew checks the selected folder before displaying the confirmation and
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
read/write/traverse permissions in the file manager. Moxcrew does not change
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
`workspace-write` with approval policy `never`. Supported Claude and Gemini
headless adapters also provide bounded start/cancel control; compatible CLI
diagnostics still do not prove account/model access or quota.

Create a stable Project and explicitly link one or more registered worktrees
from its card after adding them.
Existing unlinked worktrees stay usable. Project Memory stores messages and owner
sources for new bound tasks/teams; it does not silently import historical work or
provider transcripts. Select a project to read its bounded source page, filter by
keyword, enter a message or revise a directly entered message. Captured task/team
sources preserve their original text and project. Unlinking/unregistering a worktree
does not erase history. Source records alone are context; authoritative ADRs
are managed in Decisions. Use the accepted-decision search for free Local keyword results. In Cloud, Find a published decision opens an explicit search with query-upload disclosure, semantic/fallback status and an exact downloaded-copy action. Compatible subscription Cloud service deployment is required for remote search. Scoped Local agent tools and optional automatic drafts are available.

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
| macOS | `~/Library/Application Support/Moxcrew` |
| Windows | `%APPDATA%\Moxcrew` |
| Linux | `$XDG_DATA_HOME/moxcrew` or `~/.local/share/moxcrew` |

An explicitly set absolute `MOXCREW_DATA_DIR` selects an alternate directory;
`PATCHFLEET_DATA_DIR` remains a legacy alias. Different simultaneous values
are rejected. On upgrade, the Host opens a recognized prior default profile
in place. Multiple candidate profiles require an explicit selection. Back up
the stopped profile before upgrade. The old Windows Local AppData Patchfleet
folder is an installer destination, not a canonical data profile.
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
cargo run -p moxcrew-core --bin moxcrew -- --help
cargo run -p moxcrew-core --bin moxcrew -- --data-dir /absolute/test-data recover
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
quoting are platform-specific; Moxcrew does not rewrite provider settings.

## Upgrade and uninstall

Close the application, back up its data, and install a reviewed compatible
replacement. Uninstall the application through the OS while keeping owner data
for recovery. Delete that directory only through an explicit owner action.
