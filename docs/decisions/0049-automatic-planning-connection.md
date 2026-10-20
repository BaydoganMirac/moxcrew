# ADR0049: Automatic planning connection

Status: Accepted by owner correction; Task0051

Date: 2026-10-20

The owner rejects making provider selection part of the required Chat flow.
Mox uses the existing selected-provider planner to interpret the request and
propose the skills and crew. Choosing infrastructure is not an owner's task
before describing a goal.

By default, resolve a supported usable connection from existing diagnostics in
stable Codex, Claude Code, Gemini CLI order. Do not claim an ability or cost
ranking that has not been measured. Keep the current readiness checks and one
bounded diagnostic read. An optional manual override lives in secondary controls;
an unavailable explicit override must remain visible rather than silently switch.

Resolve and freeze the actual provider before sending a request or creating its
Project. Exact receipts, selected files, prior context and explicit Prepare/Start
remain authoritative. Automatic selection does not imply an automatic retry on
another account or execution of a crew. No model can interpret the request until
at least one provider is connected; the product does not bundle model access.

This supersedes ADR0048's immediately visible provider picker, while preserving
its conversation, receipt, privacy and execution boundaries.
