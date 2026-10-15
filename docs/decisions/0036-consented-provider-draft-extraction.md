# ADR 0036 Consented provider draft extraction

Status: Accepted for approved Task0035

Date: 2026-10-15

Use the installed supported Codex CLI for optional Local source-to-ADR drafting,
with explicit project consent, bounded jobs and no automatic acceptance. This
reuses the existing provider account rather than adding an OpenAI API client or
copying credentials. Local storage does not imply offline inference; the UI must
disclose that sources are sent to the provider and may consume its usage.

V7 records settings, durable source/version job identity, attempts/outcomes and
generator provenance. Successful completion invokes existing V4 proposal
commands under the owner's explicit delegation in one transaction; V4 meanings
and owner-only acceptance/supersession remain unchanged. The derived exact view
labels generated drafts and supporting source excerpts. Provider output is
untrusted: validate schema, source/project/version and exact excerpts; questions
may yield no ADR, and potential conflicts never replace an accepted rule.

Automatic processing begins with sources created/edited after enablement. Older
sources require explicit queueing. Disabling, source edits and shutdown cancel
work; failed/interrupted attempts require deliberate retry. Run one short-lived,
tool-disabled, read-only provider process in a scratch directory, with bounded
stdin/stdout and no repository cwd. Keep the existing writer and native runtime;
add no model dependency, background database or Cloud data path.

References: [Task0035](../plans/0035-automatic-sourced-drafts.md),
[ADR0021](0021-project-scoped-local-decision-memory.md),
[Codex non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode),
[CLI reference](https://learn.chatgpt.com/docs/cli/reference).
