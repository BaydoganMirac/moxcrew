# Automatic planning connection

Updated: 2026-10-20

Status: Implementing; integrated verification pending.

Plan: [Task0051](../plans/0051-automatic-planning-provider.md).
Decision: [ADR0049](../decisions/0049-automatic-planning-connection.md).

## Product flow

The default composer takes the request and Send, without requiring a provider
choice. Mox uses existing diagnostics to resolve a supported usable local
connection in stable Codex/Claude Code/Gemini CLI order. The real planner then
interprets the goal and proposes abilities and crew members. This does not claim
a measured model-quality ranking, bundled model access or offline inference.

Provider selection and Automatic remain optional secondary controls. An explicit
unavailable override cannot silently use another account. If no usable connection
exists, keep the message and offer Connections. Existing installation, version,
authentication and capability checks remain authoritative.

Resolve and freeze the actual provider before the first Send/Project creation.
Changes to diagnostics, preferences or late receipts cannot retarget the frozen
request or trigger cross-account inference retries. Original goals, labelled
clarification context, scoped files and exact Project receipts retain their
contracts. Prepare and Start remain explicit owner actions.

## Verification

Integrated tests, independent review, actual native captures and development
package proof pending. No core, event schema, Cloud boundary or dependency change.
No live inference, login, installation, push or deployment. Native Windows/Linux
and live model/account behavior remain unverified.
