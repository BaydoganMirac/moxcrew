# V0 paired Cloud control

Status: Done

Last updated: 2026-07-16

## Done

- Owner authorized completion of Phase 3.
- Phase 1 observation and Phase 2 durable local control are complete.
- Accepted ADR 0015 for outbound polling, allowlisted projection, and remote
  `cancel_run` only.
- Implemented stable-installation pairing, owner-only credential storage,
  heartbeat, bounded allowlisted projection, intent polling, durable receipt
  retry, revocation handling, and local pairing/status UI.
- Added explicit truncation flags so Cloud never presents the first 32 active
  work items or runs as a complete view.
- Passed 128/128 public tests and the public production build.
- Passed the real cross-repository HTTP pairing, projection, cancel, receipt,
  revocation, and denial-after-revocation smoke.
- Completed independent cross-boundary review with no remaining P0-P2 finding.

## In progress

- None.

## Next up

1. Keep production auth, transactional storage, deployment, and packaging in
   separately approved milestones.
2. Choose the next product milestone before expanding the remote intent set.

## Blockers

- None.

## Open questions

- Production auth, deployment, and retention are deliberately outside
  this undeployed Phase 3 alpha and require their own approved milestone.
