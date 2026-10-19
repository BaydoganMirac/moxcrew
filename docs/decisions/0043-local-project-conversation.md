# ADR 0043: Durable Project conversation and copied revision inputs

Status: Accepted for owner-authorized Task0041

Date: 2026-10-19

## Decision

Render a Project-scoped conversation from durable owner job commands and work
facts. Preserve raw owner goals and provenance rather than parsing composed
provider instructions or treating status text as model-generated replies.
Actual result files remain scoped explicit reads in the native artifact panel.

V9 owner audit adds project_chat_send and desktop_motion while retaining V1 queue/run, V3 Project
binding and V8 managed-job/export meanings. Authorize managed jobs from their
exact applied inner queue proof and preceding owner request, including V9.
Recover a known queue result after a crash without rerunning inference or copies.
Older readers need a stopped pre-upgrade backup after V9 commands are used.

Follow-up work keeps the same Project and uses a fresh managed directory.
Owners explicitly choose up to eight output references from one terminal job.
Copy bounded validated bytes into inputs/ with a SHA-256 manifest; reject
cross-project, linked, unavailable, oversized or actively written source files.
Original outputs and accepted decisions remain unchanged. No implicit global
context or cross-provider fallback is introduced.

Provider connection diagnostics are ephemeral local reads with fixed commands,
bounded output and safe status fields. They neither read credential files nor
prove model access merely from an installed or authenticated CLI. Existing
unsupported Claude/Gemini execution stays visibly unavailable until proven.

Keep the native single-writer architecture, Cloud allowlists and opt-in sharing.
Conversation bodies, selected files, skill content and provider diagnostic data
are not added to ordinary Cloud projection. Guided connections and expressive
motion do not authorize new API credentials, runtimes or external actions.

## References

- [Task0041](../plans/0041-connected-project-conversation.md)
- [ADR0042](0042-owner-providers-default-skills-and-motion.md)
- [Managed jobs](0041-managed-project-job-directories.md)
