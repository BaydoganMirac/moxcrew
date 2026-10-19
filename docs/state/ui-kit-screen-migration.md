# UI kit across native screens

Updated: 2026-10-19

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0049](../plans/0049-apply-ui-kit-to-screens.md).
Component contract: [Moxcrew UI kit](../../design/moxcrew/UI-KIT.md).

The shared kit now reaches all nine native destinations and their forms rather
than only the Project work surfaces. The chat-centered Crew/Outputs context
remains the primary work flow. No core, provider execution, Cloud protocol,
dependency, schema or version change is included.

| Surface | Applied change |
|---|---|
| Chat | Shared message/task cards, regular choice controls, connection/catalog/detail dialog frames |
| Projects | 24/20 px page/section hierarchy, metric/project/folder cards, measured action before long names, project/folder forms |
| Teams / Crew | Plan and role cards, roster inputs, provider/recipe selectors, revision forms and action footers |
| Tasks | Intake inputs, task cards, action rows and compact pagination |
| Memory | Source and semantic-search fields, source cards, filters and extraction controls |
| Decisions | 44 px filter fields/selectors, evidence/edit forms and selected decision cards |
| Agents | Catalog card hierarchy, metadata, pack configuration and installation form |
| Providers | Diagnosis card hierarchy and connection controls |
| Account / Cloud | Optional sign-in/pairing forms, account cards and shared search dialog |
| Outputs / delivery | Preview/revision forms, private setup, file filter/delivery rows and frozen approval |

Regular form controls are 44 px, popup options/dense actions 36 px; cards and
dialogs use 16 px insets/radii. Page padding is 24 px, 16 px at narrow/short
windows. Metadata uses 12 px. The Turkish optional-login label is shortened to
keep the full sidebar inset. Long Project names use the width remaining after
the measured action; actual clicks retain their exact Project and unsent draft.
Quick start places the provider action before longer supporting text.
The Project dialog uses its form's single card frame. When Crew is open beside
Chat, the conversation shows a compact plan summary; the full roster and its
Prepare/Edit actions belong to the Crew context. Closing the context restores
the full chat card without changing the draft or approval behavior.

Widget identities, private-input undo clearing, explicit Prepare/Start, frozen
file/hash/revision approval and exact receipt handling remain intact. Intentional
mascot/icon/status geometry and the measured unframed composer inset are retained;
read-only folder wells/feedback use the kit's compact inset.

## Verification

- Complete all-feature desktop suite: **171 passed, 0 failed, 0 ignored**.
  Strict workspace/all-target/all-feature Clippy and workspace formatting pass.
- Actual rendered regressions cover long Project names with exact navigation,
  the sidebar inset, one roster/Prepare/Edit while Crew is open, actual Chat-tab
  restoration of the same draft/plan, and the right Prepare button's containment
  and exact Project/revision/goal/roster payload. Private token clearing, frozen
  approvals, file versions, receipts and keyboard behavior remain green.
- All nine destinations have native minimum-window TR/EN light/dark captures
  and normal-window TR dark captures. Additional scenes cover Project,
  Connections, Roster and frozen app dialogs, populated Decisions, Crew/Outputs,
  the ability catalog and delivery selection. The catalog/delivery scenes and
  final Project/Crew corrections were captured separately from the base matrix.
- **82 accepted native states** from 71 base captures and 20 final follow-ups;
  nine affected base captures are explicitly superseded. The combined index
  records each image's source commit, binary hash and image hash rather than
  claiming that two binaries are one. Evidence is in the sibling output
  directory `branding/moxcrew/screen-kit-qa/qa.json`; individual matrices are
  under `native/qa.json` and `native-final/qa.json`. Final fixture binary SHA-256:
  `2984a97d0adf5594c8e94f7a7caf078c5433296b9329f10559a8fb1d51b4b173`.
- Final source: `e7989ac`, following `b2559fd`, `cbc5ca3`, `9628bbf` and `8bdf028`.
- Independent source and actual macOS image review passed with no remaining
  blocker, including the final catalog, single-frame Project form, single-plan
  context and delivery selection. The reviewer did not run duplicate tests or captures.

## Development package

`dist/native/screen-kit/Moxcrew_0.12.0_screen-kit_aarch64.dmg`

Version remains 0.12.0. The separate macOS ARM64 development package is ad-hoc
signed and not publicly notarized. Strict signature verification, license
resources, exclusion of fixture/gallery code, normal isolated launch/close,
writer-lock release and DMG integrity verification pass. Evidence:
`dist/native/screen-kit/qa.json`.

- Executable: 16,555,152 bytes; SHA-256
  `ed7731604e75ed45bd6dcfae8dbde3963620fd6189c59e771e7caaadff0135b6`.
- DMG: 8,526,614 bytes; SHA-256
  `48b09fe6f41b1028ba9794267df1502c57a3c4359b94c9a5c76f28c722d714f5`.

Native scenes and package checks use disposable local data without inference or
real account writes. Native Windows/Linux execution, live providers and public
signing are outside this UI change. No push, deployment or installation is performed.
