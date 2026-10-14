# ADR 0028 Native folder selection and truthful access feedback

Status: Accepted

Date: 2026-10-14

The owner requires a folder picker instead of path typing, useful workspace
errors and clearer setup. Keep the existing canonical Git-worktree boundary.
The native dialog allows the OS to mediate user-selected folder access. It does
not override filesystem ACLs, grant provider processes broad access or enable
macOS App Sandbox. This application remains an unsandboxed native development
App; security-scoped bookmark persistence is not introduced for it.

Use pinned `rfd` 0.17.2 platform dialogs. macOS uses AppKit and Windows uses
native Shell dialogs. Linux uses the XDG portal/desktop fallback rather than
bundling GTK. Create the async dialog on the UI thread and await it off-thread.
Reuse the existing async runtime and repaint only when the result arrives.

Folder validation reports safe missing/not-directory/not-Git/unsafe/access
errors for local registration and picker preflight. The existing provider-control
validator still maps failures to `WORKSPACE_NOT_ALLOWED`, preserving operational
V1 receipts. No new event schema or Cloud fields are introduced. Registration
revalidates after preflight for a new command; a chosen folder is not proof of
future access. An existing registration with a terminal receipt can reconstruct
an exact canonical-path retry without rechecking the live folder. The unchanged
full intent comparison still rejects reused identities with changed payloads or
time bounds. Raw aliases need normal live resolution. No completed receipt is
rewritten merely because a folder became inaccessible or was removed.

Render localized feedback with a clear title, recovery and optional technical
code. Retry keeps the original command identity; changing folders starts a new
owner action. Permission guidance uses the OS selection/settings workflow,
never silent chmod, ACL edits or Full Disk Access requests.

## References

- [RFD platform, threading and packaging documentation](https://docs.rs/rfd/0.17.2/rfd/)
- [Task 0028](../plans/0028-native-workspace-onboarding.md)
- [ADR 0024](0024-native-rust-desktop-ui.md)
