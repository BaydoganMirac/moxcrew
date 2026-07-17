# V0 closed alpha readiness

Status: Done

Last updated: 2026-07-17

## Done

- Owner approved Phase 4 completion.
- Locked the cross-repository master plan and public Task 0012.
- Accepted ADR 0016 for npm CLI packaging before a desktop shell.
- Selected the existing V1 paired loop as the release smoke; no new remote
  intent is included.
- Added the public npm CLI package with idempotent start, exact process-health
  status/stop, deterministic recovery, user-profile data, and packaged Gemini
  lifecycle ingress.
- Added macOS/Windows CI, relocatable build sanitization, reviewed tarball
  contents, and a clean install smoke through npm's generated executable.
- Verified the authenticated hosted service against the public host protocol.
- Passed 130/130 public tests, production build, package and paired HTTP checks, secret/path
  scans and dependency audits.
- Completed security review with no unresolved P0-P2 finding.
- Confirmed the first owner-operated live Cloud pairing: the local host resumed
  outbound sync after a production restart without exposing local-only data.
- Completed the product-ready Local activation surface and verified a real
  paired-host queue, start, local cancel, durable receipt, and sanitized sync
  loop. The live Cloud-authored cancel remains the next separate proof.
- Added the local workspace registry and packaged add/list/remove commands so
  a Git worktree is registered once and then selected by name. The UI resolves
  opaque local IDs and keeps paths out of Cloud.
- Passed 134/134 public tests, production build, clean package lifecycle smoke,
  and a real selected-project queue/start/cancel flow on the paired host.

## In progress

- None.

## Next up

1. Exercise the live remote cancel and receipt path with a real active run.
2. Invite the first one-to-three closed-alpha users as a separate release
   action.
3. Use alpha evidence before considering signed native installers or another
   remote command.

## Blockers

- None. Hosted project creation, secrets, migration application, and deployment
  remain owner-operated release actions outside Phase 4.

## Open questions

- Signed native installers and OS login startup wait for closed-alpha evidence.
- Declarative agent packs follow a stable external activation path; executable
  plugins and multi-agent orchestration are not part of the current alpha.
