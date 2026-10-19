# Task0048: Shared UI kit and chat-centered Project studio

Status: Complete; independently reviewed and macOS development package verified

Updated: 2026-10-19

Authorization: the owner selected “Chat centered + lightly futuristic” and the
recommended approach where files open beside the conversation. Preserve the
accepted visual system, agent orchestration and newly delivered ZIP workflow.
The owner then explicitly requested a UI kit to correct inconsistent buttons,
padding and spacing across the application. Establish shared production controls
and metrics first; use them for the chat-centered layout.

## Observable result

- Use one native UI kit for primary, secondary, ghost and destructive buttons,
  input fields, cards, spacing, corner radii and focus/disabled feedback. Preserve
  the brand's44px normal and36px compact controls and4px spacing scale. Migrate
  common helpers used throughout the app and the main chat/crew/output surfaces.
  A real native component gallery and concise design documentation expose the
  same implementation for review; no disconnected mockup or web UI engine.
- Chat is the Project's primary work surface. Crew and Outputs become contextual
  toggles rather than three equal destinations; clicking an active toggle closes
  it and returns focus to the conversation. Project switching closes the context.
- At normal/wide desktop widths, Crew/Outputs open in one bounded right panel
  with the conversation and composer still present. Reuse the actual workflow
  and Output Studio controls, not a second reduced implementation.
- At narrow widths, show a readable full-width context with an explicit Back to
  chat action. Resizing preserves unsent text, focused controls, file selections
  and frozen approvals without creating a second source of state.
- Keep the current native ink/cobalt surfaces, restrained ambient/focus accents,
  Mox and reduced motion. No extra decorative animation or fabricated activity.
- Existing legacy crew/file panels, Teams hub, provider setup, resources/tools/
  apps, scoped preview/revision, export and delivery packages remain reachable.
- Scope, hash/version, late receipt, private-token clearing, explicit Prepare/
  Start and external-action approvals retain their current checks.

## Ownership and acceptance

Coordinator owns studio.rs, chat.rs, create.rs, ui.rs, i18n.rs, main.rs,
design/moxcrew UI kit documentation/tokens, task/shared docs and fixture
capture/package integration. Builder owns crew.rs, output_studio.rs and delivery.rs
only for bounded right-panel fitting and related tests. Reviewer is read-only
after stable source and actual native captures exist. No core/runtime/Cloud change,
dependency, schema, inference, external account write, push or public release.

Verify TR/EN and both themes at800x560 and1280x820, real toggle/close/keyboard and
resize interactions, visible composer beside Crew/Outputs, reachable primary
actions and frozen Project/file/approval scope. Run the complete desktop suite,
strict Clippy/format checks, actual native captures and a separate feature-off
macOS development package. Document platform/live-provider limits honestly.

## Completion

Shared controls/tokens, real component gallery and responsive conversation
context are implemented at `85c0805`. The all-feature desktop suite passed169
tests; the final responsive inset correction passed its expanded affected
resize regression. Strict workspace Clippy/formatting,42native captures,
independent source/visual review and the separate feature-off macOS development
package pass. See [UI kit state](../state/chat-centered-ui-kit.md) for exact
evidence, hashes and remaining platform/live-provider limits.
