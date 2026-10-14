# Task 0028 Native folder selection and workspace onboarding

Status: Implemented — independent acceptance review pending

Owner direction: 2026-10-14, native workspace error screenshot; replace path
typing with a folder picker, request relevant access and improve warnings/design.

## Outcome

An owner selects a repository using the platform folder dialog, understands
whether it is missing, not a Git worktree or inaccessible, and can register it
without guessing paths. Projects have a clear folder setup action and compact
layout. Turkish/English copy describes a recovery, not a bare internal code.

## Scope and ownership

- Coordinator owns the native picker, bridge/UI/feedback, locale copy, dependency
  lock, task/ADR/state/docs, versioned development packaging and integration.
- One bounded core builder owns `providers.rs` workspace validation,
  `host.rs` registration validation call and focused core tests only.
- Reviewer inspects a stable commit read-only after integration.

Reuse the single Host writer and strict V1/V2/V3 meanings. Keep the existing
`validate_workspace` provider-control failure contract; detailed diagnostics
apply before accepting a new registration. No source reading, implicit project
link, Git initialization, permission chmod/ACL change, broad access escalation,
automatic provider turn or Cloud upload. All native tests use disposable data.
No installed application replacement, owner-state migration, push/publication.

## Acceptance

- Native cross-platform folder picker replaces registration and one-off work
  path text inputs. Cancellation preserves the draft and performs no command.
  The dialog/validation does not block background work or cause continuous idle
  repaint. The selected path is displayed read-only and can be changed.
- Folder selection uses OS consent; missing/denied/not-repository/unsafe paths
  have distinct safe messages. Denied access offers a new picker selection and
  platform settings guidance. No claim that OS/ACL access can be forced.
- Host revalidates canonical folders at registration. Root/home rejection,
  linked worktrees, command identity, expiry, revisions and Cloud privacy remain.
- Feedback distinguishes success/rejection/failure, has a dismiss action and
  relevant recovery. Editing/selecting a new folder does not retry an old action.
- Projects make folder setup obvious, collapse technical identity by default
  and fit the minimum window. New copy is Turkish/English; owner text stays raw.
- Relevant core/native tests, Rust fmt/strict Clippy and legacy contract tests
  pass. Verify actual production macOS rendering and native dialog behavior;
  record untested platform/access boundaries honestly.
- Provide a verified macOS ARM64 App/DMG, current docs/state and local commits.

## Integration evidence

- Core builder commits `ce7dd0f` and `7381a90` preserve V1 runtime error meaning
  while adding detailed registration access checks and stable canonical retries.
- 95 Rust tests and 149 legacy contract tests pass; strict workspace Clippy and
  formatting pass. No event schema, source capture or Cloud field changed.
- Actual macOS folder-sheet opening and normal close with the sheet open pass.
  A disposable Git repository at mode 0500 produces the localized read-only
  warning; only this test fixture's mode is changed/restored. Cancellation and
  invalid preflight leave canonical owner events untouched.
- Final TR/EN minimum-window form and Projects captures were inspected. Footers
  remain visible outside the scrolling body, long paths truncate with tooltips,
  and technical identities are collapsed. Final images are GL readbacks.
- The 0.5.1 production App/DMG passes signature, Info.plist purpose descriptions,
  dependency and image-integrity checks. NSWorkspace returns its own bundle/PID,
  lists a native window and normal termination releases it. Final OS pixel
  capture was unavailable (“could not create image from window”); earlier sheet
  pixels and Task 0027 production presentation were inspected, not substituted
  for a claimed final production screenshot.
- Windows/Linux native dialog and global macOS TCC deny/grant checks remain
  platform release limits. No installed App, owner data or OS privacy settings
  were changed. See [current state](../state/native-rust-desktop.md).
