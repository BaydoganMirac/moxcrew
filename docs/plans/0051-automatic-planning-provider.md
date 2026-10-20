# Task0051: Automatic planning connection

Status: Owner-approved, implementing

Updated: 2026-10-20

Authorization: the owner rejects asking the user to select a provider and wants
Mox to understand the request. Make provider selection optional infrastructure;
reuse the existing real planner to infer abilities and the crew from the goal.

## Observable result

- Default Chat shows the request composer and Send without a provider picker.
  Existing diagnostics automatically choose a supported, usable local connection
  in stable Codex/Claude/Gemini order. This is readiness selection, not an invented
  quality/cost ranking based on keywords. One connection is sufficient.
- Manual provider override and Automatic remain optional under More/details.
  An explicit unavailable override must not silently use a different account.
  Show a compact truthful active connection status; if none is usable, keep the
  message and offer Connections rather than requiring a provider form.
- Freeze the actual provider before Send/Project creation. An exact receipt,
  configuration change, diagnostic refresh or late result cannot retarget an
  already frozen request. No automatic inference retry/account switch after
  failure, no implicit crew execution, and no weakening of readiness gates.
- Preserve original goal, clarification history, scoped files, Project binding,
  drafts and explicit Prepare/Start. No core/schema/Cloud/dependency change.

## Ownership and acceptance

UI implementation owns chat.rs, conversation.rs, crew.rs, connect.rs and create.rs
under the desktop crate only. Coordinator owns i18n, smoke integration if needed,
shared docs, ADR and verification/package evidence. Review a stable local commit.

Cover only-Claude/only-Gemini, multiple/no usable connections, known bad auth or
unsupported Codex, automatic default, explicit override and return to Automatic,
frozen pending receipts/requests, and actual minimum-window TR/EN interactions.
Run the full desktop suite, strict formatting/Clippy, affected native captures
and a smoke-free macOS development package. No live inference, login, install,
push or deployment.
