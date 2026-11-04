# Moxcrew UI kit

One native component system for the application. The accepted Mox identity,
Manrope and semantic light/dark colors remain unchanged. Task0048 established
the shared controls; Task0049 applies them across the native application screens.

Read [UX-RULES.md](UX-RULES.md) before changing or reviewing any screen. It owns
the researched interaction guidance, component roles, action placement, critical
flows and usability acceptance targets. This kit records implemented primitives
and measurements; shared styling alone does not prove that a screen is usable.
The 2026-10-25 review identifies flow and hierarchy defects. Their implementation
is tracked in [Task0053 state](../../docs/state/ux-rules-implementation.md);
the research record alone does not establish remediation or usability acceptance.

## Measurements

Sizes below are logical egui points; native Retina captures can contain twice as many
physical pixels on each axis.

| Element | Rule |
|---|---|
| Spacing | 4, 8, 12, 16, 24, 32, 48 pt |
| Related controls | 8 pt gap; center vertically in one row |
| Control groups | 12 pt gap |
| Sections | 24 pt gap |
| Regular controls | 36 pt height; 12 pt horizontal button padding |
| Compact controls | 32 pt height, for contextual navigation and dense row actions |
| Chips | 32 logical point height; removal has its own 32×32 target; pill radius 16 |
| Input | 36 pt single line; 12 pt horizontal / 8 pt vertical inset |
| Card | 16 pt padding / 12 pt radius |
| Page | 24 pt padding; 16 pt at narrow window widths |
| Control radius | 8 pt |
| Focus | 2 pt visible ring, independent of hover |
| Typography | 14 pt body/control, 12 pt metadata, 20/24 pt section/page heading |
| Dialog | Card frame with 16 pt inset / 12 pt radius; menus use 8 pt inset |

## Components and hierarchy

- **Primary:** one prominent next action in each local task area. Prepare and
  Start retain separate deliberate steps. Cobalt/lavender follows the theme.
- **Secondary:** an outlined alternative, such as export or configuration.
- **Ghost:** supporting actions and disclosures. Hover has a visible surface;
  invisible hit targets do not substitute for a label or keyboard focus.
- **Danger:** explicit destructive/cancel actions use semantic danger colors.
- **Inputs:** labels above, supporting/error text below. Values retain the
  caller's length bounds, identity, private-input and validation behavior.
- **Input chips:** filename or selected output context with an independently
  named remove target. Removal edits only the scoped draft. Body text truncates
  within measured bounds; full labels remain available. Stable identities keep
  keyboard focus on the same file when another file is removed.
- **Filter chips:** selected state includes a vector check and accessible selection.
  The output toolbar shows the current type; its bounded picker contains all types.
- **Suggestion chips:** fill an optional draft; never send, prepare or start work.
- **Cards:** one consistent surface and inset. Nested cards do not accumulate
  arbitrary extra padding. Empty, unavailable and error states include text.

Use regular controls for forms and task footers, compact controls for contextual
navigation and dense actions. Do not shrink a control because a window is short;
scroll its content and keep its primary action reachable. Color is accompanied
by text; keyboard activation and reduced motion remain supported.

## Native source

[studio.rs](../../crates/patchfleet-desktop/src/studio.rs) owns the shared metrics,
palette, button variants, single-line input and card frame. Existing `control`,
`submit`, `action`, `field`, `field_hint`, `card` and Project `view_control` helpers
delegate to these components while preserving their command and test hooks.

The UI-kit reference scene uses those same controls in the native application,
including chips, disabled and keyboard-focused examples. It is compiled only with
`ui-smoke`; development fixtures/gallery code are excluded from the production
package. This is an implemented component kit, not a Figma file or a claim that
every bespoke data visualization has been redesigned.

The semantic contract is [design-tokens.json](tokens/design-tokens.json). Tests
check the main dimensions against the native implementation and exercise actual
control geometry, enabled state and layout in both themes. Existing palettes
and portable CSS colors remain available for other surfaces.

## Screen adoption

| Destination | Shared components applied |
|---|---|
| Chat | Message/task cards, connection/catalog/detail dialogs, composer controls |
| Projects | Page hierarchy, metric/project/folder cards, project/folder forms |
| Teams | Plan and role cards, roster editor, provider/recipe selectors and footers |
| Tasks | Intake form, task cards, search/action rows and compact pagination |
| Project Memory | Source/search forms, cards, disclosure and extraction controls |
| Decisions | Filters, evidence and edit forms, selected decision cards |
| Agents | Catalog cards, pack configuration and installation form |
| Providers | Diagnosis cards, headings and connection controls |
| Account / Cloud | Optional sign-in/pairing forms, account cards and shared search |
| Crew / Outputs context | Plan/revision forms, file filters, delivery rows, private setup and frozen approval |

Forms use 36 pt controls and 32 pt popup options. Long project names truncate
with hover text within the width remaining after the measured action. A short
viewport scrolls content without shrinking controls. Native evidence and acceptance results are recorded
in [screen migration state](../../docs/state/ui-kit-screen-migration.md).

The unframed chat editor retains its measured 8/6 pt inset inside the 14 pt
composer frame. Read-only folder wells and feedback use the compact 12 pt inset
and control radius. Mascot/icon painter coordinates, status dots/badges and the
brand wordmark retain their intentional geometry; they are not spacing tokens.

## Task flow adoption

The composer shows Attach at the leading edge and its stage-specific primary
action at the trailing edge. Attachment/output chips and visible conflict
resolution are outside the old clipped context scroll; extra files open in a
bounded `+N` popup. The bottom composer measures its actual content rather than
reserving a fixed 76-point context slot. Cmd/Ctrl+Enter uses the same validated
operation as the primary action. Optional connection override stays secondary.
Revision context is explicitly named; inactive follow-up context stays saved
without adding a second mode row. The editor prompts for the requested change.

Host command availability is separate from local message editing. Shared
submit/action/read helpers and automatic continuation paths respect busy state;
the Chat editor, chip removal and local context navigation stay available.
Prepare and explicit Start retain their exact scope and receipt checks.

Project context is resizable when both it and a readable conversation fit;
narrow layouts retain Back to chat and the same draft. Crew summaries expose
one stage action, outputs put preview/revision beside the selected artifact,
and Brand fields scroll above a stable Save footer with dirty/validation states.
Package selection uses a compact toolbar and a bounded selection-review popup,
so the actual file list remains visible at the minimum window size. Changed
versions remain inspectable and removable; package limits remain under More.
Existing regular/compact sizes and all production/approval contracts remain.

## Desktop composition (Task0054)

The 208-point navigation remains in place when Project context opens at normal
width; a 64-point rail is reserved for narrow windows. A 760-point conversation
column and an adaptive inspector share the main surface. Only narrow detail
surfaces need Back to chat. Welcome text, composer and suggestions share a
left reading axis. Status/connection guidance sits beneath the editor surface.

The 36/32-point desktop control density preserves 14-point body text, 32-point
chip removal and visible keyboard focus. Touch guidance stays separate. Shared
cards and inspector surfaces avoid nested outlines, canvas bloom and idle
ornament. Task0054 records visual evidence; this is not user acceptance.

## Distinctive desktop workbench (Task0056)

Task0056 supersedes the Task0054 shell measurements: normal navigation is 184
points and the narrow rail is 56 points. Navigation groups and metadata are
sentence case. The active destination uses a quiet raised surface plus one
3-point orange location signal; the same orange signal marks owner input while
cobalt marks Mox/crew work.

The empty Chat state is the one expressive brand moment: a fixed cobalt launch
pad, orange Mox stage, 36/28-point editorial headline and three direct 44-point
intent shortcuts. It uses no gradient, glow or decorative motion. At 800×560 the
hero contracts and keeps the composer footer and all three shortcuts visible.
Populated Chat, Crew and Output states deliberately return to the calm workbench
system: one reading column, semantic source/state stripes and an optional bordered
inspector. This contrast is part of the hierarchy, not a second component theme.

## Distinctive desktop flows (Task0057)

The workbench language applies to every top-level desktop route. Pages begin
with a 28/24-point title, one plain-language summary and a short orange direction
marker. Do not repeat the navigation group as a breadcrumb. Status and relevant
actions share the far edge of that header.

Collections are one-column ledgers separated by quiet rules. Each record has one
3-point semantic marker: cobalt for neutral work, orange for owner input, green
for accepted/available state, pink for rejection/failure and muted ink for
history. Selected detail may use a raised surface; unselected records do not
become a grid of rounded cards. Aggregate counts belong in one flat band.

Bounded surfaces are reserved for forms, selected inspectors and explicit guide
steps. Quick Start keeps numbers because it is a real sequence; shortcuts and
metadata do not. Metadata uses sentences rather than middle dots. Empty states
pair the Mox mark with a direct next action. The same rules apply in Turkish and
English, light and dark themes, and the 800×560 minimum viewport.
