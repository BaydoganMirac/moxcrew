# Conversation-first orchestration and provider selection

Updated: 2026-10-19

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0050](../plans/0050-conversation-first-and-provider-selection.md).
Decision: [ADR0048](../decisions/0048-conversation-first-orchestration.md).

## Product flow

Task0051 supersedes the visible provider picker with automatic connection
selection and an optional secondary override. See [current connection flow](automatic-planning-provider.md).
The implementation and verification below record the Task0050 foundation.

Chat uses one message composer, Send and an immediately visible provider choice.
Skills, single-task mode and configuration remain optional secondary controls.
The first request can create a local Project; only its exact applied creation
receipt and authoritative Project allow the frozen planning request to continue.
Opening the screen, an unrelated receipt, or an unknown outcome starts nothing.

Actual owner requests and provider-returned clarification questions appear in
Chat. Replies and revisions retain the original goal and prior labelled context
using the existing bounded planner arguments. Long context is rejected visibly
without dropping the message. Prepare and Start remain explicit, scoped actions;
Send never starts an executing crew or approves an external operation. Existing
Project/file/hash/revision/receipt and private-input checks remain in force.

## Codex diagnosis

The installed Codex reported version0.163.0 and a successful saved-login status
exit code. These metadata checks do not verify model access, quota or tools.
The desktop environment lacked shell PATH setup, while Codex was installed in
an NVM version bin omitted by discovery. Resolving the shim alone still failed
with exit127 because its Node interpreter was also outside the GUI PATH. A
child-only sibling-bin PATH made the same version check succeed. One initial
five-second version timeout was transient; later checks completed promptly.

Separately, provider selection lived inside planning details and connection
metadata was loaded only after opening Connections. Selection now belongs to
Chat, with one bounded diagnostic read and independent readiness guidance.
Known missing authentication, unsupported/malformed versions and missing
executables retain their capability boundaries.

Core discovery adds bounded numeric NVM version candidates after ordinary PATH
and existing locations. Provider child processes can use their verified sibling
Node bin without changing the application's environment or sourcing profiles.
No provider is installed, logged in, upgraded or given new access by this change.

## Verification

- Workspace core and integration suites: **264 passed, 6 existing ignored**,
  including all provider discovery, conformance and cleanup checks. The first
  workspace attempt then caught an obsolete desktop assertion forbidding the
  newly intentional entry diagnostic. Its replacement checks exactly one
  connection read on Chat entry, no repeat read and no command on other tabs.
- Complete final all-feature desktop suite: **183 passed, 0 failed, 0 ignored**.
  The 75 focused conversation/Create checks are included in that suite. Workspace
  doc-test command passes with zero examples. Strict workspace/all-target/
  all-feature Clippy, workspace formatting and diff checks pass.
- Regressions cover a first Send without a Project; exact/unknown/late creation
  receipts; preserved drafts; current provider selection and bounded diagnosis;
  real clarification/revision context, including emoji and full limits; canonical
  restoration without overwriting edits; and separate visible Prepare/Start
  clicks at minimum size in TR/EN and both themes. New conversation remains blank
  even with a sole existing Project. Exact successful transfer removes only its
  sent source draft. Clearing/replacing follow-up files changes future requests
  while in-flight requests retain their frozen references.
- Independent source review closed three findings about sole-Project adoption,
  resurrected sent drafts and stale file references in `fa4e7f4`. Provider,
  bridge, context, privacy and approval boundaries have no remaining finding.
  Actual normal/minimum native frame review also has no remaining finding.
- **40 accepted native states** cover empty, composed, questions, plan and
  unavailable-provider scenes, each in TR/EN, light/dark and normal/minimum
  windows. Of 64 captures, 24 welcome-copy frames are superseded. The combined
  sibling-output index `branding/moxcrew/conversation-first-qa/qa.json` records
  the exact source commit, binary hash, image hash and path for every accepted
  state. Questions/plan retain `ce44605`; the final welcome frames use `a17a831`.
  These are rendered fixtures without provider probes or inference.
- Sanitized actual Codex connection proof is retained under that evidence
  directory's `checks/moxcrew-0050-provider-proof.json`; SHA-256:
  `067a815263db1f899931eadbc28d0ee4001ec28cf0948b178fb26ca20ce327a9`.
  It establishes discovery, supported version and reported credential presence,
  not successful model execution. Final runtime source: `a17a831`.

## Development package

`dist/native/conversation-first/Moxcrew_0.12.0_conversation-first_aarch64.dmg`

Version remains 0.12.0. The macOS ARM64 development package is ad-hoc signed
and not publicly notarized. Strict signature verification, license resources,
absence of smoke/gallery code, isolated normal launch/close, writer-lock release
and DMG integrity verification pass. Package evidence:
`dist/native/conversation-first/qa.json`.

- Executable: 16,637,760 bytes; SHA-256
  `a779bc81c875b0778218e9357ff98971e467c78bffc62ce0810ebe0b0d76597d`.
- DMG: 8,559,828 bytes; SHA-256
  `b275b14b68618de50b5078a2877436d6283492f0fcffd4f349d7989240d0e5b8`.

No live inference, external account write, installation, push or deployment is
performed. Native Windows/Linux execution, other Node version managers, live
model/account access and public signing remain unverified.
