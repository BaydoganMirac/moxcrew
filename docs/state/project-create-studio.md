# Project Create studio

Status: Bounded foundation complete; simplified interface reviewed and locally verified

Updated: 2026-10-19

## Scope

[Task0040](../plans/0040-project-create-studio.md) is the first implementation
slice inside the accepted [1.0.0 plan](../plans/0039-version-one-release.md).
It does not narrow or complete the forty-ability release target.

## Implemented

- Forty localized, inspectable recipes across eight categories; these are
  guidance, not installed tool/model/format capability claims.
- Project-scoped queueing without Git, explicit supported Codex start/cancel,
  fresh managed folders for retry and preserved original project identity.
- Scoped real output listing, bounded UTF-8 Markdown/text/CSV preview and native
  save-as export with no overwrite. Outputs stay separate from provider/input
  scratch and canonical data.
- Native Create catalog/composer and separate project-creation view. Drafts,
  receipts, results and export requests retain exact project/job scope.
- Prompt-focused startup for both empty and populated installations: one goal
  field, eight deliberate type choices and Prepare task. The catalog and
  optional name/instructions open on demand. Blank names derive from the goal
  without splitting Unicode scalar boundaries. No recipe is selected silently.
- Primary navigation is Create/Projects/Tasks; More opens the other six screens.
  Queued work shows explicit start or provider setup. A prepared task is never
  started automatically.
- Missing/replaced directories retain visible job history and an actionable
  issue, without exposing stale files or allowing an unsafe start.

Preview is limited to 256 KiB; export to 100 MiB. Listings cap at 200 results,
2,000 examined entries and five directory levels. Other file types can be
exported within limits, but this slice does not embed Office/media editors.
Result list/read/export currently work on Unix. Windows rejects these operations
with `PROJECT_RESULTS_UNSUPPORTED_PLATFORM` until pinned path-handle protection
and native verification exist. Windows task lifecycle support is not result access
proof. [Task0060](../plans/0060-atomic-owner-exports.md) now stages Unix exports
privately and atomically publishes the complete file without overwriting a
destination that appears concurrently. A failed write exposes no partial public
copy.

## Persistence and trust

[ADR0041](../decisions/0041-managed-project-job-directories.md) defines the exact
managed directory and V8 owner-command facts. Existing V1 run/queue and V3
project facts retain their meanings. Ordinary coding work retains Git checks.
Older code without V8 support cannot read a data directory after these commands;
back up before using this development build and restore an offline pre-upgrade
copy when rolling back. Never delete or rewrite V8 log lines for rollback.

Cloud allowlists/protocol and inference credentials are unchanged. Sources,
recipes, result text and paths are not added to ordinary Cloud projection.
No new provider account, key, executable dependency or hosted compute was added.

## Evidence so far

- Workspace tests passed; the final simplified desktop run passed all 114 checks.
  One additional core authorization-order regression also passed. Core checks
  cover managed prepare/start/cancel, actual Host-to-fake-provider output,
  terminal state and restart, scoping, replay, retry, traversal, links, bounds
  and export collision. Loopback tests ran with local fixture permission.
- Twenty final macOS GL captures cover Create/results in TR/EN, both themes,
  normal/minimum windows and project-free first use using disposable data and
  no provider probes.
  Independent review accepted integrated code `2dd3483`, including crash-prefix
  receipt recovery, fresh retry after deleted directories, fail-closed Windows
  result access and replacement-file preservation on export failure. Follow-up
  review accepted `5b7f04b` with no remaining actionable findings in the prompt
  flow, title derivation, draft carry, explicit execution and secondary routes.
- Strict workspace/all-target/all-feature Clippy and a feature-off release build
  passed. The isolated macOS App has a verified ad-hoc signature and bundled
  licenses. Final captures supersede the initial interface evidence.
- Development-only presentation and edited-video feasibility artifacts were
  produced. They do not establish a bundled/shipped renderer or a live AI result.

## Remaining

[Task0041](connected-project-conversation.md) supersedes this initial prompt
layout with a durable Project conversation, copied selected-file follow-ups,
connection diagnostics and saved reduced motion. Its separate evidence records
the current interface; this file preserves Task0040's original acceptance.
Live installed-Codex inference, tool-dependent ability quality, native Windows/
Linux behavior and the rest of the 1.0.0 pipeline remain unverified. No push,
public package/signing, production installation, Cloud deployment or migration
has occurred.

## Native views

- [Create, Turkish/light](../screenshots/create-studio/create-tr-light.png)
- [Create, English/dark/minimum](../screenshots/create-studio/create-en-dark-min.png)
- [Results, Turkish/dark/minimum](../screenshots/create-studio/results-tr-dark-min.png)

## Development package

The feature-off App was launched through NSWorkspace with disposable data and
failing provider stubs. It closed normally and released the writer lock. Its
six-second startup sample was 109,360 KiB RSS and 0.14 seconds cumulative CPU;
this excludes model work and is not a sustained benchmark. WindowServer image
capture returned “could not create image from window”; the images above are
actual GL captures, not an independent OS capture of the production window.

- `dist/native/create-studio/Moxcrew_0.12.0_create-simple_aarch64.dmg`:
  7,443,801 bytes, SHA-256
  `c49646ee33bf1888d3ce4f547ad9fbfc96027bbbcb1a708ba901080096c3bb02`.
- Packaged executable: 14,248,320 bytes, SHA-256
  `fd5f28507cebd27d3f63ea74bd2267bbec787a309016213aadde4dce10351d3c`.

Ad-hoc signature, bundled licenses, production absence of smoke controls and
DMG integrity passed. The installed application remains untouched. This is a
local development artifact with V8 backup/rollback requirements, not 1.0.0 or
a publicly signed/notarized release.
