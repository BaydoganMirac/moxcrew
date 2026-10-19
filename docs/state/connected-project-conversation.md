# Connected Project conversation

Status: Bounded implementation complete; independently reviewed and macOS package verified

Updated: 2026-10-19

## Scope and behavior

[Task0041](../plans/0041-connected-project-conversation.md), authorized by
“Onaylı başla”, implements part of the accepted 1.0.0 baseline. It does not
reduce or complete the eight-area, forty-ability, ten-workflow release target.

- Project conversation derives durable raw owner goals and honest job state
  from canonical commands. It does not invent assistant replies or import
  provider transcripts. Reads are chronological, scoped and paged (20 default,
  50 maximum); old V8 Create jobs and retries retain their owner goal.
- The native composer prepares work; execution still requires explicit start.
  Chat/Sohbet is the entry screen; Projects and Tasks remain primary navigation.
  An on-demand side panel reads actual output files using the existing bounded
  preview/export contract. Follow-up references come from one completed, failed
  or interrupted managed job in the same Project.
- Up to eight files and 25 MiB total are pinned, validated and copied into a
  fresh inputs/ directory with a SHA-256 manifest. Originals are preserved.
  No-reference follow-ups retain identity when the old job folder is missing.
  Inputs are task material, not accepted decisions or access grants.
- Connections displays installation, saved authentication and supported control
  separately. Explicit fixed-argument diagnostics have bounded time/output and
  expose no raw output, account identity or credential path. They never log in
  or make a model request. Codex/Claude credentials do not establish model
  entitlement or quota; Gemini authentication remains unknown.
  Installation/update guidance explicitly requires a Moxcrew restart before
  checking again: the running controller retains its resolved program paths.
- Only Codex executes work. Project-bound jobs require Codex 0.155.1+; ordinary
  control's older minimum does not establish Project-memory compatibility.
  Claude/Gemini execution remains release work.
- Eight self-authored, Apache-2.0 version1 skill guides attach to the selected
  real job. They specify produce/revise/verify rules, including unavailable-tool
  reporting. These are instruction/workflow groundwork, not installed renderers
  or forty verified abilities.
- Visible active work drives bounded Mox motion. Idle/reduced motion schedules
  no animation repaint. The language/motion preference file uses schema2 and
  reads schema1; updates preserve the other preference.

## Persistence and privacy

[ADR0043](../decisions/0043-local-project-conversation.md) adds V9 owner
requested/receipted audit for project_chat_send and desktop_motion. V1 queue/run,
V3 Project binding and V8 audit semantics retain their meanings. Crash recovery
returns a proven inner queue result without repeating uncertain copies or
inference. Raw selections remain canonical; the manifest is a mutable job file,
not a canonical input-hash fact.

Retry verifies/copies the prior job's selected inputs and exact manifest bytes;
it never rereads changed grandparent outputs. Missing, inconsistent or unsafe
inputs reject before a fresh job is queued. A coherent edit to both a writable
manifest and its file cannot be detected against canonical hashes in this
version. The manifest does not establish authenticated provenance.

Back up the stopped complete data directory before using V9 commands. Older
development builds reject V9; rollback restores that unchanged copy, including
preferences. Never delete or rewrite audit lines to downgrade.

Cloud serializers and protocol remain unchanged. Boundary canaries cover raw
conversation goals, diagnostics and reduced-motion settings. Selected inputs,
recipe/guide content, results and paths remain local by default.

## Evidence

- Provider diagnostics commit `30e89a9` independently reviewed with no actionable
  findings. Six diagnostic fixture tests and eight existing conformance tests
  passed; descendant timeout cleanup independently passed.
- Core commit `3fb10d5` passed the full core suite, including five conversation,
  ten creation, actual Host-to-fake-provider output/follow-up, preference legacy/
  restart/idempotency and Cloud allowlist checks. Installed-provider/Cloud
  PostgreSQL fixture checks retain their explicit ignored gates.
- Independent runtime review found a referenced-input retry loss. Fix `26f0b26`
  passed seven conversation and ten creation checks plus strict core Clippy,
  and independent review closed that finding. The retry/restart proof deletes
  grandparent output and still retains the exact selected input/manifest bytes.
- Integrated native code `4e7b283` and correction `4fcd06e` passed independent
  review with no remaining actionable P1/P2 findings. The correction preserves
  selected files across manual skill changes, including delayed receipts, and
  adds explicit CLI installation/update restart guidance.
- Full workspace/all-feature tests passed: 331 passed, four existing explicit
  ignored gates. After the final UI correction, all 126 desktop checks passed.
  This includes actual control clicks, scoped history/files/exports, minimum
  dialog containment, drafts/async receipts and idle/reduced/hidden repaint
  behavior. Local sign-in fixture tests require loopback permission.
- Formatting and strict workspace/all-target/all-feature Clippy passed. The
  final UI correction passed strict desktop Clippy with ui-smoke. No production
  dependency or executable environment was added.
- Thirty-five final macOS native GL captures against `4fcd06e` cover the empty
  conversation, durable history, actual fixture files, connection dialogs,
  TR/EN, both themes, normal/minimum windows and a saved reduced-motion scene.
  These queued-job fixtures do not establish live model output or active GL
  animation timing; the bounded motion scheduler is covered by UI unit checks.

## Native views

- [Project conversation, Turkish/dark](../screenshots/project-conversation/chat-tr-dark.png)
- [Actual files beside chat, English/light](../screenshots/project-conversation/files-en-light.png)
- [Connections, Turkish/dark/minimum](../screenshots/project-conversation/connections-tr-dark-min.png)
- [First use, English/dark/minimum](../screenshots/project-conversation/first-use-en-dark-min.png)

## Development package

The feature-off release App has a verified ad-hoc signature, bundled licenses
and no smoke controls. An isolated NSWorkspace launch with disposable data and
failing provider stubs found its own native window, closed normally and released
the writer lock. The six-second startup sample was 112,880 KiB RSS and 0.14
seconds cumulative CPU; it excludes model work and is not a sustained benchmark.
WindowServer capture again returned “could not create image from window”; the
views above are actual GL captures, not an independent production-window image.

- `dist/native/connected-conversation/Moxcrew_0.12.0_project-chat_aarch64.dmg`:
  7,530,975 bytes, SHA-256
  `4f67b62d9754802a4c17bc5f766dece94de325fc2f8470cf57d43e30209b67e5`.
- Packaged executable: 14,446,448 bytes, SHA-256
  `5c7994c48ca39598cf17e44b5040f621c0726e8cbe4f7238777da08ce3a36fd5`.

DMG integrity passed. The installed application remains untouched. This is a
local macOS ARM64 development artifact with V9 backup/rollback requirements;
source remains 0.12.0, not 1.0.0 or a signed/notarized public release.

## Limits

Windows result access and file-reference follow-ups fail closed until equivalent
secure native file handling is implemented. Text-only follow-ups remain usable.
No live provider inference, entitlement, complete ability quality, native
Windows/Linux runtime or hosted-service deployment is established here.
No global skill install, API key creation, provider login, push, public package,
production installation or Cloud migration/deployment is performed by this task.
