# Moxcrew UI kit

One native component system for the application. The accepted Mox identity,
Manrope and semantic light/dark colors remain unchanged. Task0048 established
the shared controls; Task0049 applies them across the native application screens.

## Measurements

Sizes are logical UI pixels; native Retina captures can contain twice as many
physical pixels on each axis.

| Element | Rule |
|---|---|
| Spacing | 4, 8, 12, 16, 24, 32, 48 px |
| Related controls | 8 px gap; center vertically in one row |
| Control groups | 12 px gap |
| Sections | 24 px gap |
| Regular controls | 44 px height; 16 px horizontal button padding |
| Compact controls | 36 px height, for contextual navigation and dense row actions |
| Input | 44 px single line; 12 px horizontal / 10 px vertical inset |
| Card | 16 px padding / 16 px radius |
| Page | 24 px padding; 16 px at narrow window widths |
| Control radius | 12 px |
| Focus | 2 px visible ring, independent of hover |
| Typography | 14 px body/control, 12 px metadata, 20/24 px section/page heading |
| Dialog | Card frame with 16 px inset/radius; menus use 8 px inset |

## Components and hierarchy

- **Primary:** one prominent next action in each local task area. Prepare and
  Start retain separate deliberate steps. Cobalt/lavender follows the theme.
- **Secondary:** an outlined alternative, such as export or configuration.
- **Ghost:** supporting actions and disclosures. Hover has a visible surface;
  invisible hit targets do not substitute for a label or keyboard focus.
- **Danger:** explicit destructive/cancel actions use semantic danger colors.
- **Inputs:** labels above, supporting/error text below. Values retain the
  caller's length bounds, identity, private-input and validation behavior.
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
including disabled and keyboard-focused examples. It is compiled only with
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

Forms use 44 px controls and 36 px popup options. Long project names truncate
with hover text within the width remaining after the measured action. A short viewport scrolls content
without shrinking controls. Native evidence and acceptance results are recorded
in [screen migration state](../../docs/state/ui-kit-screen-migration.md).

The unframed chat editor retains its measured 8/6 px inset inside the 16 px
composer frame. Read-only folder wells and feedback use the compact 12 px inset
and control radius. Mascot/icon painter coordinates, status dots/badges and the
brand wordmark retain their intentional geometry; they are not spacing tokens.
