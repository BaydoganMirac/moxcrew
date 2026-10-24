# UI/UX research and design reference

Updated: 2026-10-25

Status: Research and durable reference complete; runtime UI remediation pending.

Reference: [Moxcrew UX rules](../../design/moxcrew/UX-RULES.md).
Implemented primitives: [UI kit](../../design/moxcrew/UI-KIT.md).

## Scope and evidence

The owner requested detailed UI/UX standards research, understandable Moxcrew
flows and a durable reference, including an explanation of Rust/egui limitations
and the absence of useful chip patterns. This session changes documentation
only. No runtime, token, dependency, schema, version, authority or Cloud change
is included.

The reference distinguishes normative WCAG web requirements, informative native
mapping, platform guidance, research heuristics and Moxcrew design targets.
Primary sources are linked beside the relevant claims and dated 2026-10-25.
ISO coverage is limited to official abstracts; full paid standards were not read.

Source review used working tree `5e07ee1`, whose latest runtime change is
`072f371`. It covered shared kit controls, composer attachment/selection/action
layout, the fixed Project context panel, Create busy gating and Brand form
action order. An existing minimum-window TR/dark attachment fixture capture
was also inspected; it shows partial clipping below the selection-conflict row.
This was an existing fixture, not a new live-user study or full capture audit.

Findings include a constrained 76-unit composer context, oversized attachment
rows without semantic input chips, entire-Create busy disabling, a fixed
600-unit context panel at a 1280-unit threshold, and Save brand preceding its
fields. Rust does not forbid the needed controls; egui exposes custom widget
and interaction APIs. App-specific behavior, semantics and layout remain the
implementation's responsibility.

## Durable follow-through

- AGENTS.md requires the reference and UI kit for future UI/UX changes/reviews.
- The UI kit and documentation index link the reference and distinguish
  implemented metrics from design targets.
- The reference defines concrete chat → plan → Prepare → explicit Start → real
  output → selected revision flows, component roles, contextual action placement,
  accessibility targets, state/recovery behavior and blocking acceptance defects.
- Current accepted Project/provider/receipt/Prepare/Start/side-effect contracts
  remain authoritative. Suggested UI remediation requires an approved bounded
  implementation task card; the research is not recorded as shipped UI.

## Verification and remaining evidence

Documentation links, source references, changed-file scope and whitespace were
checked. No Rust suite or package build was rerun for documentation-only work.
No moderated user session, new native platform run, screen-reader audit or
accessibility conformance assessment was performed. Existing technical and
fixture passes establish their stated behavior/geometry only.

Next: use the reference's P0 composer/selection/busy slice to define the next
implementation task, then verify the complete request-to-result flow with
representative content and users. Runtime defects remain open.
