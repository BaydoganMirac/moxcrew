# First-task quick start

Status: Complete for approved implementation; independently reviewed and locally verified

Updated: 2026-10-19

## Owner approval

The owner selected easier first use and explicitly approved
[Task 0038](../plans/0038-first-task-quick-start.md) implementation on 2026-10-19.

## Implemented

- A nonmodal Mox guide connects project creation, actual folder linkage,
  observed Codex compatibility and a deliberate first task start. It opens
  automatically for empty Projects; populated installations keep their view.
  Owners can leave/reopen it and return from Tasks/Providers.
- Existing project/folder forms, native picker, provider refresh, composer,
  revision gates and command/receipt behavior are reused. Multiple projects
  or folders require explicit selection; missing selected IDs are not silently
  replaced. Opening an unsent task preserves its text and folder.
- Progress comes from saved facts, not UI clicks. An applied start, its exact
  run and frozen project context prove completion. Folder registration alone,
  queueing, failed/uncertain starts and another project's run do not complete
  the guide. Checks use a fixed number of linear history passes.
- The guide requires the existing stable Codex 0.155.1 memory-compatible
  minimum for project-bound work. Ordinary control capabilities are unchanged;
  compatibility does not claim provider authentication.
- Generic task-queue navigation avoids attributing historical work to a
  project just because its folder matches. This matters after a folder is
  relinked: its old work retains the original frozen project. Setup never
  creates a provider turn, retries a mutation or changes another draft.
- TR/EN, both themes, static bundled Mox, minimum-window next actions and
  immediate task/provider recovery after a start error are implemented.
  Saved facts survive restart; unsent draft text remains session-only.
- No dependency, core event/schema, Cloud/protocol, authentication, account
  requirement, credential handling or production provider setting changed.

## Checks and independent review

The full workspace run passed 271 checks with four existing opt-in checks
ignored. After the UI follow-up, all 98 desktop checks pass; the 175 unchanged
core checks from that run remain the core evidence. Eight new guide checks
cover exact saved completion, multiple/stale scope, provider minimum,
navigation/draft preservation, folder cancel/error/link, leave/reopen,
historical foreign/unbound work and minimum-window failure recovery.

Strict workspace/all-target/all-feature Clippy, formatting, feature-off release
build, app signature verification, bundled licenses, absence of smoke controls
and DMG integrity pass. An initial sandbox-only desktop rerun denied existing
loopback fixtures; the same tests pass with local-server permissions. No test
timeout or runtime behavior was changed to bypass that boundary.

Independent review accepted `805281e` plus `b923ff2` with no remaining actionable
finding. The reviewer independently ran 97 desktop checks before the follow-up
and all eight final guide checks. The folder-only prior-task recommendation
was removed after review; the missing checkmark glyph and minimum-window error
recovery were corrected after visual inspection.

## Native evidence

Twenty actual native GL captures exercise project, folder, provider, task and
failure scenes at 800x560 in TR/EN and light/dark. Setup/queue facts use a real
disposable Host; task-scene compatibility and failure feedback are clearly
illustrative UI fixtures. No installed provider or model turn was used.

- [Folder setup, Turkish/light/minimum](../screenshots/quick-start/folder-tr-light-min.png)
- [Provider check, English/dark/minimum](../screenshots/quick-start/provider-en-dark-min.png)
- [First task, Turkish/dark/minimum](../screenshots/quick-start/task-tr-dark-min.png)
- [Failure recovery, English/light/minimum](../screenshots/quick-start/failed-en-light-min.png)

The separately packaged production App was opened through NSWorkspace with an
empty temporary data directory and failing provider stubs. Its own bundle/PID
and native window matched; an actual
[macOS WindowServer capture](../screenshots/quick-start/production-macos-tr.png)
was inspected. Normal termination exited and released the writer lock. At six
seconds, its empty-startup sample was 114,784 KiB RSS and 0.18 seconds cumulative
CPU. This excludes real provider work and is not a sustained usage guarantee;
canonical history still determines retained-state costs.

## Development artifact

This is a separate 0.12.0 development build with the existing package/data
identities, not a new public release. The installed application was untouched.

- `dist/native/Moxcrew_0.12.0_quick-start_aarch64.dmg`: 7,318,788 bytes,
  SHA-256 `6e256d5fc428aab961086fa4a88317fcb99bbe2d5d58065ac7a41a2f12ece88e`.
- Ad-hoc-signed App executable: 13,968,160 bytes,
  SHA-256 `d132cfe3f2be45b3140f7e5f332caedff3e34cf0da9f42331dd2c1e5f084f455`.

## Remaining evidence

Moderated-user activation benefit, live provider sign-in/first model turn,
native Windows/Linux runtime and public signing/notarization remain release
checks. Setup was verified with disposable data rather than owner projects.
No push, deployment, production migration or installed-app replacement occurred
in this implementation task. Use an external clean-install session to measure
completion, elapsed time, hesitation and failures before claiming improved
activation. Local operation and existing Cloud boundaries remain intact.
