# External beta activation

Status: In progress

Last updated: 2026-07-18

## Prepared and verified

- Owner approved Task 0017 and the release-preparation boundaries.
- Fast-forwarded local `main` to tested release commit `6d33332`.
- Created the local `release/local-beta-0.1.0` branch without rewriting the
  unavailable public upstream history.
- Completed public package metadata, a protected manual beta-publish workflow,
  and the first-release/trusted-publisher runbook.
- Pinned the future OIDC publishing job to Node 24.14.0 and npm 11.5.1 with
  `id-token: write`; the first package creation remains an interactive owner
  release because npm cannot attach a trusted publisher before the package
  exists.
- Removed build cache, diagnostics, traces, type output, and source maps from
  the distributable. The final beta dry run contains 212 files and is roughly
  0.8 MB packed and 2.9 MB unpacked; build identifiers can shift exact bytes.
- Made clean-package smoke independent of a user's global npm cache and proved
  clean install, workspace registration, doctor, start, status, stop, and
  recovery.
- Passed the 135-test Public suite on Node 24, the production build, package
  smoke, npm publish dry run, and the paired Public/Cloud HTTP smoke.

## Open release gates

1. Restore a reviewable public upstream branch and run the checked-in macOS,
   Windows, Linux, Node 22, and Node 24 GitHub jobs. The public push was
   attempted and GitHub rejected it because the current OAuth credential lacks
   permission to update workflow files; the remote branch remains unchanged.
2. Approve and perform the initial interactive npm beta publication with the
   owner's npm account and 2FA. Configure the trusted publisher immediately
   afterward; never pass an OTP or long-lived npm token through a command.
3. Approve the hosted Cloud release, then complete one live browser-to-host
   `cancel_run` and definitive receipt proof.
4. Approve invitations and run five comparable moderated activation sessions.

## Release boundaries

- Local `main` contains the release. The public GitHub push was rejected and no
  npm package was published.
- No hosted migration, deployment, DNS change, secret mutation, or invitation
  was performed.
- Plugins, agent packs, multi-agent orchestration, billing, and new remote
  intents remain outside this evidence phase.
