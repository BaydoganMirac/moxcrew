# Task 0017: External beta activation

Status: Approved

Owner approval: 2026-07-17

## Goal

Prove that a target solo builder can install Patchfleet, reach the first useful
local view, pair Cloud, understand remote state, cancel one supported Codex run,
and recover without repository access or owner intervention.

## Why this is next

The Local beta code and package lifecycle are complete, but external activation
is not proven. Agent packs, executable plugins, and multi-agent orchestration do
not meet their entry gates until this baseline exists.

## Milestones

### A. Release candidate hygiene

- Restore a reviewable public upstream/branch without overwriting the current
  worktree.
- Run the Node 22 macOS, Windows, and Linux package jobs plus Linux Node 24.
- Verify install, doctor, workspace registration, start, status, stop, upgrade,
  uninstall, backup, and recovery from a clean package.
- Prepare npm trusted publishing and provenance; do not publish without a
  separate owner release approval.

### B. Live V1 proof

- Pair a clean Local host with the deployed Cloud Free candidate.
- Start a real supported Codex run, request `cancel_run` from Cloud, and record
  the definitive local and Cloud receipt.
- Verify retry, restart, revocation, offline Local use, and forbidden-data
  canaries around the same path.

### C. Five moderated activation baselines

- Use five target solo builders on clean macOS, Windows, or Linux installs.
- Record manual timestamps and failure points from discovery to first local
  view, signup to first sync, and remote request to terminal receipt.
- Record misunderstood copy, recovery attempts, privacy concerns, repeated
  setup patterns, notification interest, and willingness to keep using Cloud.
- Fix only P0-P2 defects and repeated activation blockers during the phase;
  keep one-off feature requests as evidence, not scope.

### D. Evidence decision

- Rank activation failures and the recurring supervision job.
- Update public state and the reduced roadmap with verified findings only.
- Decide whether the next implementation card is Cloud notifications/history,
  declarative Local agent packs, or release hardening.

## Acceptance criteria

1. The existing 135-test suite, production build, clean-package smoke, and
   paired HTTP smoke remain green.
2. The configured cross-platform package jobs pass from a reviewable branch.
3. One live browser-to-host `cancel_run` ends in an exact terminal receipt and
   no forbidden data crosses the boundary.
4. Five moderated sessions produce comparable task-flow records and a ranked
   blocker list; no invented conversion target is used before these baselines.
5. Patchfleet remains fully usable with Cloud unavailable.
6. The next product slice is selected from observed evidence and receives its
   own approved task card.

## Boundaries

- No new remote intent, protocol version, arbitrary shell, Cloud path alias,
  executable plugin loader, multi-agent orchestrator, billing, or analytics
  vendor.
- Local-only telemetry remains opt-in. Moderated session notes contain no
  source, prompt, transcript, credential, provider output, or absolute path.
- A native installer or login service is added only if the baselines prove
  manual Node startup is a repeated blocker.

## Approval boundary

Approving this card authorizes preparation, verification, defect fixes within
scope, and moderated beta work. It does not authorize npm publication, live
Cloud migration, deployment, DNS changes, or external invitations; each
requires explicit owner direction at the point of execution.
