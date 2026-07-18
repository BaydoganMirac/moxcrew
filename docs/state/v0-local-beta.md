# V0 publishable Local beta

Status: Done

Last updated: 2026-07-18

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
- Completed approved Task 0018's Local product correction: returning owners now
  see current work, new task intake, and plain-language activity first;
  completed onboarding disappears and provider/session detail is progressive.
- Added monotonic projection upgrade recovery. A changed sanitized view at the
  same local event count advances once, while genuinely stale state still
  fails closed. The live paired host recovered from revision 73 to 74 and
  resumed successful sync.
- Replaced the broken Turbopack production build path after it generated an
  incomplete routes manifest; the standard Next production build now starts
  through the packaged CLI and serves the verified operational console.
- Passed 137 Public tests, the production build, Local browser review, and the
  real paired HTTP cancel/receipt smoke on the corrected tree.

## Next up

1. Run the same package lifecycle through the configured GitHub macOS, Windows,
   and Linux jobs.
2. Deploy the matching Cloud product commit, then complete the deferred live
   Cloud-authored cancel with the owner.
3. Test a clean install with external beta users before native installers or
   background login services.

## Release boundaries

- No package was published and no repository was pushed.
- No live Cloud migration or deployment was performed.
- Agent packs, executable plugins, and multi-agent orchestration remain behind
  their documented evidence gates.
