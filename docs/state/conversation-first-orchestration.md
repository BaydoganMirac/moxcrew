# Conversation-first orchestration and provider selection

Updated: 2026-10-19

Status: Implementing; final integrated verification pending.

Plan: [Task0050](../plans/0050-conversation-first-and-provider-selection.md).
Decision: [ADR0048](../decisions/0048-conversation-first-orchestration.md).

## Product flow

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

Integrated tests, independent review, native captures and package proof pending.
No live inference, external account write, installation, push or deployment.
Native Windows/Linux behavior and live model/account access remain unverified.
