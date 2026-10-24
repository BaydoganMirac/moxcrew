# UX rules implementation

Updated: 2026-10-25

Status: In progress; owner-approved implementation

Plan: [Task0053](../plans/0053-apply-ux-rules-and-chips.md).
Reference: [UX rules](../../design/moxcrew/UX-RULES.md).

## Implemented source

- Shared 32-point removable input, selected filter and optional suggestion chips;
  named keyboard targets, stable attachment identity and bounded `+N` overflow.
- Direct Attach and trailing stage action; visible selection/attachment conflict,
  bounded 64-point text editor and same-operation Cmd/Ctrl+Enter.
- Current-content composer reservation uses component metrics and measured
  wrapped feedback, not egui Panel's previous natural height. A regression
  demonstrated that cached height moved More between press/release after file
  panel closure. The existing full follow-up/skill-switch journey now passes.
- Bridge busy gates commands and automatic receipt/read continuations while
  preserving local typing, removal and navigation. Exact pending draft and
  Project/provider/resource/receipt checks remain authoritative.
- Resizable Project context reserves readable conversation width; narrow
  layouts retain the same draft. Crew stage footers, semantic output filters,
  contextual Preview/Revise and secondary export/package actions are integrated.
- Brand values precede a stable Save footer with dirty/invalid/no-change states.
  Existing selected logo references outside a loaded resource page are retained.
- TR/EN labels, shared UI kit scene and semantic chip token are updated.

## Verification so far

All-feature desktop suite: **224 passed, 0 failed**. Strict workspace/all-target/
all-feature Clippy and formatting pass. Local account fixture tests required
loopback binding outside the network-restricted sandbox; no live account/model
was used. Long-message and panel-transition regressions cover 100/125/150% scale.
Native capture review and a local development package remain in progress.

This is UI implementation evidence, not a moderated usability study or native
screen-reader/conformance assessment. No dependency, protocol, backend authority,
schema, version, provider installation or implicit execution change is included.
