# Task 0038: First-task quick start

Status: Approved; implementation in progress

Updated: 2026-10-19

Owner approval: 2026-10-19, explicitly approved this Hızlı Başlangıç plan and
requested implementation.

## Observable outcome

A new Local user can create a project, connect an existing Git folder, check
Codex compatibility and deliberately start a first task without discovering
the navigation alone. Local account creation and Cloud pairing are optional.

## Evidence and hypothesis

The current native Projects screen provides a first-project form and then a
folder-link action. Work creation and explicit start live in Work; provider
diagnostics live in Providers. These controls exist, but no continuous guide
connects them. Task0028 and Task0036 already delivered folder selection and
first-project presentation; this card builds on those completed controls.

Hypothesis: keeping the next useful action visible reduces first-task setup
confusion. No moderated-user result or activation improvement is claimed.

## Proposed interaction

Use the Moxcrew theme and a small static Mox illustration. Display `Quick start`
in English and `Hızlı Başlangıç` in Turkish, with four compact steps:

1. **Project:** open the existing project form. An applied project creation
   exposes the project in the canonical snapshot before this step completes.
2. **Folder:** open the existing native folder registration/link controls for
   that explicit project. A selected path alone does not complete this step;
   the registered workspace must actually be linked to the project.
3. **Codex:** show observed availability/compatibility and link to Providers
   plus the existing explicit refresh. Compatibility is not proof of provider
   authentication. Observation-only Claude/Gemini cannot satisfy execution
   readiness. Missing, unsupported and failed observations have a useful next
   action through existing diagnostics; no automatic installation or login.
4. **First task:** open the existing composer with the selected linked folder.
   Explain the goal and instruction fields with short examples. Queueing and
   `Start Codex` remain separate deliberate actions. Completion requires an
   applied start and its corresponding run, never a click or queued item alone.

Automatically offer the guide in the empty Projects state. Keep it nonmodal;
owners can leave it and reopen it from Projects. Existing populated users are
not redirected. With multiple projects/folders require an explicit selection
and preserve that target by ID within the session. Do not overwrite an unsent
task or silently move it to a different folder.

After restart, rebuild progress from existing saved facts. Unsent drafts are
session-only in this slice; do not claim they were saved. Folder cancellation,
failed registration/link, stale revision and failed start leave the relevant
step incomplete with the current recovery action. Do not chain another mutation
or retry just because the UI rerenders, opens a step or receives an error.

## Ownership and inputs

One coordinator owns this bounded native UI slice and handoff:

- `crates/patchfleet-desktop/src/ui.rs`: guide, navigation, progress and inline
  behavior tests; reuse existing forms and commands rather than copying them.
- `crates/patchfleet-desktop/src/i18n.rs`: TR/EN interface text; owner content
  stays unchanged.
- `crates/patchfleet-desktop/src/main.rs`: only disposable `ui-smoke` scene
  setup if needed for visual checks; production startup stays unchanged.
- This card, `docs/state/first-task-quick-start.md`, `docs/state/README.md`, and
  the activation guidance in `docs/install.md` at implementation handoff.

Inputs: Task0028, Task0036, the existing folder/preflight, WorkDraft, command
identity/revision/receipt behavior, provider capability gate and Moxcrew assets.
If those inputs cannot support the outcome, report the concrete gap before
widening this card. No new bridge command or core contract is assumed.

## Acceptance

- Empty-project through linked-folder and queued/start states have a visible
  correct next action in TR/EN, light/dark and the native 800x560 minimum.
- Focused UI checks cover successful progression, cancel/failed folder setup,
  unsupported Codex, failed start, explicit multi-project/folder scope and
  preservation of an existing draft. Rerender, navigation and restart snapshots
  do not enqueue/start/retry work or fabricate completion.
- Existing folder, project, work, memory and command feedback tests pass.
  Run `cargo test --workspace --all-features`, workspace formatting and strict
  all-target/all-feature Clippy using the established local Rust toolchain.
- Inspect actual disposable native frames for the four guide stages and a
  failure state in both languages/themes; record platform checks not run.
- Deliver a feature-off macOS development App/DMG using the existing packaging
  path, without replacing the installed application. Update state and provide
  a focused local commit with checks and remaining risks.
- Follow with a moderated clean-install task before claiming better activation;
  record first-run completion, elapsed time, hesitation and failure location
  without adding telemetry or inventing a numeric improvement.

## Boundaries and rollback

No new dependency, model call during setup, demo task, Git initialization,
automatic provider turn, provider execution expansion, persistence schema,
Cloud/auth/billing/protocol behavior, credential handling or data upload. No
deployment, publication or installed-app replacement is part of this card.

The guide is presentation over existing commands. Reverting its UI changes
restores the previous forms without data migration. Stop or revise the design
if it hides a recovery, loses owner text, chooses the wrong scope or starts a
run without an explicit owner action.
