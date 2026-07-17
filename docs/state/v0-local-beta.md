# V0 publishable Local beta

Status: Done

Last updated: 2026-07-17

## Done

- Owner approved Task 0016 and accepted ADR 0018.
- Replaced the end-of-life Node 20 package floor with Node 22 and added one
  Node 24 compatibility job.
- Added Linux to the macOS and Windows CI release matrix.
- Added dependency-free `patchfleet doctor` checks for runtime, owner storage,
  durable state, provider projection, registered projects, and optional Cloud
  state without printing credentials or event contents.
- Extended the clean tarball lifecycle through doctor, workspace management,
  start, status, stop, and recovery.
- Completed task-oriented install, upgrade, uninstall, recovery, backup, and
  provider-compatibility documentation.
- Passed the 135-test Public suite on Node 24 after the focused diagnostics
  correction, the production build, and clean package lifecycle smoke.
- Re-ran the real two-repository HTTP smoke through pairing, sanitizer,
  Cloud-authored cancel, definitive host receipt, revocation, and offline Local
  operation.

## Next up

1. Run the same package lifecycle through the configured GitHub macOS, Windows,
   and Linux jobs.
2. Complete the deferred live Cloud-authored cancel with the owner.
3. Test a clean install with external beta users before native installers or
   background login services.

## Release boundaries

- No package was published and no repository was pushed.
- No live Cloud migration or deployment was performed.
- Agent packs, executable plugins, and multi-agent orchestration remain behind
  their documented evidence gates.
