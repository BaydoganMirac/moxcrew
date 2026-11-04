# Task0056: Distinctive desktop workbench

Status: Implemented and locally verified; owner visual judgment pending

Updated: 2026-11-04

Authorization: the owner rejected the general Moxcrew design and UX, explicitly
limited the work to the desktop application, and requested the Anthropic
`frontend-design` direction.

## Goal

Make the native desktop product feel like a deliberate Moxcrew workbench rather
than a generic dashboard. Preserve the accepted Mox identity, native egui
runtime, conversation-first flow, exact Project/provider/file/receipt checks,
and explicit Prepare then Start authority.

## Design direction

- Keep the existing semantic brand palette: ink/deck/raised surfaces, paper
  text, cobalt action and orange signal.
- Use Manrope as one intentional family with a clear 36/28/24/20/15/12 scale.
- Compose one compact navigation, one readable workstream and one contextual
  inspector. The composer shares the workstream's left reading axis.
- Let the empty-state Mox and goal canvas be the memorable visual moment. Keep
  the rest quiet, structured and information-led.
- Give that moment one bold cobalt launch pad, one orange Mox stage and an
  asymmetric editorial lockup. It is the single high-energy composition; the
  workstream remains calm enough for long sessions.
- Remove decorative all-caps labels, repeated card framing, duplicated headings,
  ambient glow, generic gradients and ornamental motion.

Wide composition:

```text
navigation | Project + Chat / Crew / Outputs
           | workstream                 | contextual inspector
           | composer                   | only when requested
```

Narrow composition keeps the existing single-detail behavior and the same
draft, Project and exact artifact selection.

## Scope and acceptance

Change the shared native presentation and the primary Chat/Crew/Outputs journey:

- shell/navigation density and hierarchy;
- project context header and view navigation;
- empty welcome, conversation surfaces and composer;
- proposal, mission and output presentation where shared primitives reach them;
- light/dark and minimum/normal window behavior.

Add no dependency, browser UI, backend contract, event/schema change, provider
behavior, automatic execution, Cloud mutation, installation or publication.
Existing keyboard names, visible focus, reduced-motion posture, draft recovery,
exact receipts and file revisions remain mandatory.

Validation requires focused component/layout tests, the desktop test suite,
formatting and strict Clippy. Native captures must be reviewed when the local
capture path is available; technical checks do not claim moderated usability or
owner visual acceptance.

## Result

The native shell now uses a 184-point navigation or 56-point rail, sentence-case
groups, orange selection signals and a quieter raised active destination. Chat
opens with the Mox launch pad, aligned composer and direct intent shortcuts.
Conversation, plan and mission surfaces share one semantic source/state stripe;
the Project inspector and Chat/Crew/Outputs switch use the same tighter geometry.

The feature has no new runtime dependency and no Cloud, core protocol, provider,
event, schema, automatic execution, installation or publication change. All 230
desktop tests pass and strict workspace Clippy/formatting pass. Accepted native
captures cover dark Turkish normal, light English minimum and a populated plan.
These checks establish implementation evidence, not the owner's visual approval.
