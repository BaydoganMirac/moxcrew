# Project Create studio

Status: Core and native interface implemented; final review/render verification in progress

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
- Missing/replaced directories retain visible job history and an actionable
  issue, without exposing stale files or allowing an unsafe start.

Preview is limited to 256 KiB; export to 100 MiB. Listings cap at 200 results,
2,000 examined entries and five directory levels. Other file types can be
exported within limits, but this slice does not embed Office/media editors.

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

- Workspace tests passed, including 106 native desktop checks. New core checks
  cover managed prepare/start/cancel, actual Host-to-fake-provider output,
  terminal state and restart, scoping, replay, retry, traversal, links, bounds
  and export collision. Loopback tests ran with local fixture permission.
- An actual macOS GL Create frame was captured using an empty disposable data
  directory and no provider probes. Final compact layout/theme/language evidence
  and independent review remain in progress at this checkpoint.
- Development-only presentation and edited-video feasibility artifacts were
  produced. They do not establish a bundled/shipped renderer or a live AI result.

## Remaining

Complete native render matrix, reviewed release build and independent review.
Live installed-Codex inference, tool-dependent ability quality, native Windows/
Linux behavior and the rest of the 1.0.0 pipeline remain unverified. No push,
public package/signing, production installation, Cloud deployment or migration
has occurred.
