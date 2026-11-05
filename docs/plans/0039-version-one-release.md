# Task 0039: Moxcrew 1.0.0 master release

Status: Owner-approved release target; first implementation slice in progress

Updated: 2026-10-19

Owner acceptance: the broad ability plan is accepted for 1.0.0. Public
[ADR 0040](../decisions/0040-version-one-release-baseline.md) records this
baseline. This plan defines technical deliverables; private launch strategy and
copy remain outside this repository. No release date is committed.

## Objective

One general-purpose Project can take owner-provided sources through research,
documents, presentations, video, images, audio, data and supported web/actions,
with Mox coordinating composed work. Deliver all eight areas in one 1.0.0
release. Forty abilities and ten composed workflows below are the release
baseline, not an assertion that they work in the current application.

Project remains the user-facing scope. Reuse the existing project/decision,
queue, provider, single-writer, receipt and native UI foundations. General work
must not require a Git repository, branch or terminal knowledge. Existing coding
execution and stored data must remain compatible.

The owner reaffirmed local Codex, Gemini CLI and Claude Code as integration
targets, with at least one connected provider required for inference. First-use
connection, strong default skills and expressive state-driven motion are release
requirements under [ADR0042](../decisions/0042-owner-providers-default-skills-and-motion.md).
Current Claude/Gemini observation does not establish their execution support.
The conversational UI proposal must preserve Project scope and actual artifacts.

## Ability acceptance matrix

All rows start **planned / unverified**. For each row record the implementation
commit, supported inputs/outputs, required tool/provider version, actual test
command, output evidence and remaining limits in release state. A failed,
unsupported or unavailable path must be visible rather than reported complete.

| ID | Required ability | Observable acceptance result |
|---|---|---|
| R1 | Source-backed web research | Retrieved sources, access dates and traceable claims in an exportable report. |
| R2 | Comparison | Owner criteria applied to a comparison table; recommendation and uncertainty distinguishable. |
| R3 | Multi-document synthesis | Attributed findings from multiple supported documents, including disagreements. |
| R4 | Claim/source checking | Supported, disputed and unresolved claims linked to inspected evidence. |
| R5 | Source change tracking | Two snapshots yield a dated, meaningful change report; no-change is handled. |
| D1 | Word documents | A generated report/proposal/CV opens as editable DOCX with preserved owner facts. |
| D2 | PDF processing | Supported text/OCR extraction and merge operations produce readable, correctly ordered output. |
| D3 | Brief and action plan | Notes become an attributed brief, missing questions and actionable tasks. |
| D4 | Translation/rewrite | Supported translation or style revision preserves meaning and source version. |
| D5 | Template filling | Supported template fields are populated; missing values remain explicit. |
| P1 | Presentation creation | Research/source material becomes editable PPTX with actual slide renders. |
| P2 | Presentation editing | A requested local edit changes the intended slide while retaining other content. |
| P3 | Branded presentations | A supplied template/brand is applied with readable, checked layout. |
| P4 | Visual storytelling | Charts/infographics communicate source data accurately within a presentation. |
| P5 | Narrated presentation | Speaker notes and timed narration yield a playable presentation video. |
| V1 | Video editing | Requested cuts, joins and order produce a playable video and retained edit recipe. |
| V2 | Short clips from long media | Content-based clip selections have inspectable time ranges and usable rendered clips. |
| V3 | Captions and aspect versions | Timed captions and supported horizontal/vertical versions survive playback checks. |
| V4 | Video composition | Titles, images and audio are placed as requested with a playable mixed result. |
| V5 | Motion/explainer video | A brief produces an actual animated promo/explainer file and revisable source. |
| I1 | Image generation | Selected provider returns a usable image matching the requested dimensions/content. |
| I2 | Image editing | Requested edits/background operations preserve the intended reference and original. |
| I3 | Batch image conversion | All selected inputs convert/resize within stated constraints, with per-file failures. |
| I4 | Thumbnail/poster/social assets | Requested layouts and sizes produce readable, usable image files. |
| I5 | Brand application | Project brand assets are consistently applied across supported image materials. |
| A1 | Transcription | Supported media yields inspectable transcript and timed subtitles where requested. |
| A2 | Speech generation | Approved text yields playable narration through the selected supported access path. |
| A3 | Audio cleanup/leveling | Processed audio meets specified level/cleanup goals without losing the original. |
| A4 | Podcast editing | An edit plan yields playable segments, chapter information and a packaged result. |
| A5 | Translated audio version | Meaning and timing are checked in the translated/narrated output; no voice-cloning claim. |
| T1 | CSV/XLSX cleaning | Converted/cleaned files reopen; missing values and transformed fields are attributable. |
| T2 | Calculation/trend analysis | Numerical conclusions agree with an independent deterministic calculation. |
| T3 | Data charts/reports | Charts and report values reconcile with the underlying dataset. |
| T4 | Budget/scenario models | Editable formulas and scenario changes recalculate correctly in the supported format. |
| T5 | Structured data extraction | Supported sources yield a traceable table with uncertainty/missing fields visible. |
| W1 | Website creation | Generated landing/portfolio files have a working preview and export. |
| W2 | Interactive tools and coding | A bounded tool/prototype executes its requested behavior; existing coding stays usable. |
| W3 | Browser tasks | A supported research/form task uses a scoped session and reports the actual outcome. |
| W4 | Connected-app actions | Supported mail/calendar/note/task operations use the intended account and confirmed scope. |
| W5 | Scheduled composed work | A scheduled job runs at the intended time, handles unavailable hosts and avoids duplicates. |

For W4, the exact initial application/operation coverage must be stated before
implementation; generic connector availability does not prove every action.
R5 measures change detection; W5 measures scheduling. All work uses scoped data
and access. Newly encountered source/skill text cannot grant permissions.

## Ten composition proofs

Each proof must run in a real Project, expose intermediate output dependencies
and allow a targeted revision without losing prior versions. A required failed
step must prevent an overall completed claim; partial usable outputs stay visible.

| Proof | Required composition |
|---|---|
| J1 | Research → presentation → site → promo video → image/content package |
| J2 | Recording → transcript → selected clips → captions → covers/text |
| J3 | Sources → learning guide → quiz → illustrated/audio/video explanation |
| J4 | Dated travel research → alternatives → budget → itinerary/PDF |
| J5 | Dataset → checked analysis → charts/report → presentation → audio brief |
| J6 | Job/source facts → tailored CV/letter → portfolio → interview material |
| J7 | Product sources → images → catalog/copy → ad variants → video |
| J8 | Event inputs → budget/plan → invitation → information page → presentation/clip |
| J9 | Multiple papers → evidence/disagreement table → synthesis → visuals → explainer |
| J10 | Meeting source → proposed decisions/tasks → follow-up draft → explicitly authorized app action |

These are technical composition tests; no automatic purchase, public publication
or third-party message follows from composing artifacts. Owner acceptance alone
makes a proposed project decision authoritative.

## Work packages and ownership

One coordinator owns shared contracts. The paths below bound implementation
areas; create a detailed card for each concrete change before mutation. Overlap
is coordinated sequentially, not treated as independent file ownership. Existing
helpers win over speculative new frameworks. Internal work packages are not
separate public launches.

| Package | Owned areas | Dependency / exit evidence |
|---|---|---|
| F0 Runtime/access decision | `docs/architecture.md`, `docs/decisions/`, `docs/plans/0039-version-one-release.md` | Choose a native-compatible execution/tool-access path using real presentation and video feasibility evidence; record environment/provider/installation costs and constraints. |
| F1 General work and sources | `crates/moxcrew-core/src/work.rs`, `host.rs`, `domain.rs`, `store.rs`, `project_memory.rs`, `crates/moxcrew-core/tests/` | After F0: source-scoped non-Git work, access validation, cancel/restart, retained coding behavior. |
| F2 Skill/tool availability | `crates/moxcrew-core/assets/`, `crates/moxcrew-core/src/providers.rs`, concrete skill implementation under `crates/moxcrew-core/src/`, related core tests | After F0: curated/versioned skills, local import, provenance, tool/permission preflight and explicit unavailable states. |
| F3 Artifacts and native results | Concrete artifact implementation under `crates/moxcrew-core/src/`, `crates/moxcrew-desktop/src/studio.rs`, `ui.rs`, `command.rs`, `bridge.rs`, `i18n.rs` | After F1: project-bound result versions, preview/open, export, targeted revision and preserved original files. |
| F4 Documents/data/presentations | Selected concrete tool adapters under `crates/moxcrew-core/src/`, approved assets, core tests and fixtures | After F0/F2/F3: D1–D5, T1–T5, P1–P5 output and revision evidence. |
| F5 Media | Selected concrete tool adapters under `crates/moxcrew-core/src/`, approved assets, core tests and fixtures | After F0/F2/F3: V1–V5, I1–I5, A1–A5 real media output, playback/dimension/timing and revision checks. |
| F6 Research/actions/scheduling | Selected concrete adapters, `crates/moxcrew-core/src/work.rs`, `teams.rs`, related core tests | After F0/F1/F2/F3: R1–R5 and W1–W5, account/session scope, idempotency and offline scheduling behavior. |
| F7 Crew/onboarding integration | `crates/moxcrew-core/src/teams.rs`, pack/template assets, desktop `studio.rs`, `ui.rs`, `i18n.rs` and related tests | After producing paths exist: J1–J10, source/decision reuse, nontechnical setup and honest progress. |
| F8 Release hardening | `Packager.toml`, existing CI workflows, `docs/release.md`, installation/recovery docs and release evidence | All scope rows: install/upgrade/recovery and supported OS matrix; reviewed candidate and rollback proof. |

Task0040 selects existing supported Codex control and exact managed directories
for the first non-Git job/result foundation under ADR0041. Development-only
editable PPTX and edited MP4 feasibility checks passed with existing host tools;
their redistribution licenses, shipped integration and cross-platform execution
remain unresolved. Forty localized recipes are guidance, not F2 tool availability
or forty completed abilities. See [Create state](../state/project-create-studio.md).

Owner-approved Task0041 adds durable Project request history, selected-file
follow-ups/retry preservation, eight versioned guides, explicit local-provider
diagnostics and bounded reduced-motion support. This is additional F1/F2/F3
groundwork; it does not complete tool availability, verified artifact renderers
or the ability/composition matrix. See
[conversation state](../state/connected-project-conversation.md).

No shipped presentation/video tool environment has been chosen yet. Reuse candidates
that require Python, Node or a headless browser do not silently change ADR 0024.
A changed execution environment needs an accepted architecture record before
relying on it. Model credentials, paid services and dependency downloads are not
created or enabled by this master plan.

## First actionable card: F0 feasibility and contract

Objective: choose one supported way to run both an actual editable presentation
and a real video edit for a non-Git Project through bounded local authority.

Owned files: this plan, `docs/architecture.md`, the next numbered architecture
decision under `docs/decisions/`, and `docs/state/version-one-release.md`.
Use disposable fixture/evidence directories outside owner data. No runtime
contract is widened just to make a demo work.

Inputs: ADRs 0024, 0038–0040; current core/provider/work/team contracts; the
existing native release and commercial boundaries. Engine research lives in
the private plan and does not establish that an integration works.

Acceptance: record exact versions/environment, installation steps, a PPTX that
opens/renders, an edited video that plays, and the commands/output checks used.
Record task-to-tool isolation, cancellation/recovery design, model access needs,
supported OS implications and package/resource cost. Compare compatible options
and select the smallest path meeting both tasks. Until measured, report unknowns.

Forbidden scope: changing user settings or secrets, weakening Git validation,
adding a background execution authority, installing global tools silently,
deploying Cloud, licensing commitments or announcing release readiness.

Handoff: feasibility evidence, chosen architecture/remaining decisions, detailed
F1/F2/F3 cards, local commit and updated release state. Owner acceptance of this
release baseline supplies product scope; do not ask again about the eight areas.

## Definition of 1.0.0 ready

- Clean setup succeeds independently with each of the three supported local
  providers: detect/install guidance, sign-in, capability check, a real job,
  cancellation, restart and scoped revision. Users need only connect one.
- Default skills include compatible tools/templates and actual output checks;
  capability evidence distinguishes provider/tool/account limitations. Preserve
  all forty abilities rather than counting catalog metadata as implementation.
- Connection/work/handoff/result motion follows actual state, supports reduced
  motion and keyboard navigation, and does not introduce continuous idle redraw.
- All forty IDs have real input → output/action and revision/failure evidence;
  all ten composition proofs pass using the release candidate.
- Non-Git first use, supported provider/tool setup, clear costs/access and TR/EN
  native UI work without requiring an account or Cloud subscription for Local.
- Project isolation, explicit decision acceptance, file preservation, cancel,
  restart, denial/revocation and idempotency hold across supported paths.
- No new default source/media/prompt upload to Cloud; operational protocol and
  optional sharing remain explicit. Cloud unavailability does not break Local.
- Run relevant boundary checks plus `cargo fmt --all -- --check`,
  `cargo test --workspace --locked` and
  `cargo clippy --workspace --all-targets --locked -- -D warnings`.
  Add actual tool/output checks; the Rust suite alone cannot prove media quality.
- Follow [release](../release.md) for native OS/install, signing, upgrade,
  backup/rollback and artifact checks. A configured CI matrix is not native proof.
- For advertised Cloud paths, complete the existing private commercial
  transition, compatible deployment, production provider and receipt evidence.
- Accepted launch message, real demo/example outputs, setup material and honest
  feature/cost/data copy are prepared in the private release plan.

Do not bump the current package to 1.0.0 merely to reserve the target. A reviewed
candidate determines coordinated manifest/lockfile/package/UI version changes.
Push, public signing/publication, deployment and migrations remain explicit
release actions. No 1.0.0 build or runtime change is delivered by this document.
