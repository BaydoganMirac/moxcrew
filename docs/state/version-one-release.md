# Moxcrew 1.0.0 release state

Status: Scope approved; productive crews and bounded production implemented, complete ability/release evidence pending

Updated: 2026-11-04

## Approved baseline

The owner accepted the broad ability plan for version 1.0.0. See
[ADR 0040](../decisions/0040-version-one-release-baseline.md) and the
[master plan](../plans/0039-version-one-release.md). Eight areas, forty abilities
and ten composed project workflows belong to the same release. Presentation
and video paths are core scope. Private launch messaging is accepted separately.

## Evidence ledger

| Area | IDs | Current status |
|---|---|---|
| Research | R1–R5 | Frozen sources and bounded text extraction implemented; full research quality proof pending |
| Documents | D1–D5 | DOCX creation/paragraph revision, attributed briefs, source-pinned translation/style rewrite with protected facts, and exact template filling; real LibreOffice open proof; PDF merge and broader semantic/layout quality pending |
| Presentations | P1–P5 | Editable PPTX creation/slide revision; real LibreOffice open proof; full area pending |
| Video | V1–V5 | Real three-aspect packages, burned captions/covers, frame/range revisions; full area pending |
| Images | I1–I5 | Fixed local ComfyUI generation/reference/inpaint integration, mask/region edits, checked 2–12 source batch conversion and PNG validation; live model quality/full area pending |
| Audio | A1–A5 | Real bounded cleanup to WAV; timed whisper.cpp transcription adapter; checked ordered podcast segments to playable M4A plus chapter JSON; voice/translation/full area pending |
| Data | T1–T5 | CSV cleanup/statistics, XLSX output/open proof, exact-column charts, formula-backed scenario workbooks, and literal-evidence structured extraction with explicit uncertain/missing cells; broader quality proof pending |
| Web/actions | W1–W5 | Explicit app actions, isolated static public-page/form-draft browser and offline interactive tools; live accounts/full area pending |
| Composed workflows | J1–J10 | Reusable crews, model planner, review/delivery, local schedules and checked multi-file delivery packages implemented; full ten-workflow release proof pending |

The first [Create studio slice](project-create-studio.md) reuses Project/decision,
durable tasks, native UI and supported Codex control for managed non-Git jobs.
Forty localized recipes, scoped real output reads and explicit export are
implemented. Ordinary coding tasks/teams retain their Git workflow. A catalog
entry does not complete any row above. Source remains 0.12.0; V8 local audit
facts require the documented stopped-backup rollback procedure. Cloud runtime,
model credentials and production installation are unchanged.

[Task0052](eleven-product-capabilities.md) adds rich inputs, visual revisions,
Project style/accepted skills, bounded triggers, alternatives and literal evidence
through the same Chat. This extends the productive implementation without
claiming completion of every ability, workflow, model-quality or release gate.

[Task0041](connected-project-conversation.md) adds a durable native Project
conversation, bounded selected-file follow-ups, explicit connection diagnostics,
eight versioned guides attached to real jobs, and saved reduced motion. These
guides define produce/revise/verify rules; they do not complete any ability row.
Chat and motion commands add V9 owner audit facts with the same backup rollback
requirement. Task0044 adds Claude/Gemini execution and the bounded tool pipeline
described below.

Development feasibility produced an editable four-shape PPTX (20,027 bytes,
reopened/rendered with LibreOffice) and a reordered/cut two-second MP4 (320×180,
24 fps, 48 frames, 189,712 bytes; decoded and frame-order checked). These used
existing development tools: artifact-tool 2.8.99 and ffmpeg/ffprobe 7.1.1.
They do not establish redistributed engines, task isolation or live model output.

## Next executable work

The owner reaffirmed provider onboarding, quality default skills and animation
requirements in [ADR0042](../decisions/0042-owner-providers-default-skills-and-motion.md).
One of local Codex, Gemini CLI or Claude Code is sufficient; all three have
execution adapters. Task0044 verifies their lifecycle with protocol peers and
retains provider-reported usage without estimates. Connection diagnostics,
explicit-term skill suggestions, durable request history, selected-file revisions
and bounded motion are implemented in Task0041. Real entitlements/provider
versions and the complete forty-ability quality matrix still require evidence.

Task0044 implements frozen resource ingestion, tool readiness, native artifact
revisions, typed Office/data/media production and explicit app-action previews.
Python 3.9+, FFmpeg/FFprobe and optional PDF text extraction are owner-installed;
no third-party binary is redistributed. See [productive crews](productive-project-crews.md)
and [Output Studio](output-studio.md) for actual operation limits and evidence.
Task0047 adds [delivery packages](delivery-packages.md): searchable Project outputs,
explicit multi-file selection and a checked ZIP with provenance. It makes existing
outputs usable together; it does not complete an additional generation ability.
[Task0062](ability-readiness-matrix.md) maps every one of the forty recipe IDs to
declared required/optional tools and separately checked access, displays actual
selected-Project readiness before selection and links missing prerequisites to
existing setup. This closes the catalog-preflight visibility gap only; it adds no
live model or output-quality evidence to the area rows above.
[Task0063](start-readiness-preflight.md) carries the same evidence into Chat,
Create, Crew and Tasks start actions. Required local setup that is missing or not
yet read blocks the native Start control and routes to existing setup; optional
tools and source/model/Host access checks remain non-blocking and explicit. This
is a desktop guard, not a new Host protocol authorization or quality proof.
[Task0064](owner-system-check.md) composes the same existing connection and
selected-Project Studio reads into one read-only System check. It reports local
engine, provider, tool, image and app evidence without installing, signing in,
running a model or turning configuration into a quality claim.
[Task0065](artifact-verification-summary.md) makes the existing exact artifact
read understandable in Output Studio. It separates hash-matched version proof,
format-specific inspection and human quality review for text, Office structure,
images, media and unsupported formats. This common evidence layer does not add
live-model proof or complete any ability's content-quality acceptance row.
[Task0066](exact-revision-comparison.md) lets the owner explicitly read a
preserved parent and compare its bounded text or Office structure with the
selected exact current version. It is a local review aid, not semantic diff,
output acceptance, rendered-layout proof or new ability evidence.
[Task0067](exact-artifact-owner-review.md) records the owner's reviewed or
needs-changes decision for one exact Project/work/path/hash tuple. The Host
rechecks current bytes, restart replay is durable and a new hash remains
unreviewed. This is human evidence, not automatic content-quality proof.
[Task0068](review-aware-delivery.md) keeps that exact decision visible while
assembling a delivery package and records reviewed, needs-changes or unreviewed
beside each checked hash in the schema2 ZIP manifest. Warnings remain advisory;
the owner retains explicit export control.
[Task0069](exact-owner-review-notes.md) adds bounded local context to the exact
review decision. V16 retains V15 replay, clearing removes the active note and
delivery manifests intentionally keep only status.
[Task0070](batch-image-conversion.md) adds the missing concrete I3 batch path:
one typed request converts 2–12 exact Project images, preserves source order and
reports partial failures without discarding successful outputs. This is local
execution evidence, not image-generation/model-quality proof.
[Task0071](podcast-editing.md) adds the deterministic A4 edit path: 1–12 exact
source ranges become one checked playable M4A with output-relative chapter data.
It does not claim automatic editorial decisions, transcription or publication.
[Task0072](checked-csv-charts.md) adds the deterministic T3 chart path: exact
frozen CSV columns and an explicit aggregation produce a static SVG, plotted data
JSON and matching concise report. It does not claim statistical interpretation.
[Task0073](editable-scenario-workbooks.md) adds the deterministic T4 model path:
exact frozen CSV columns produce an editable Base/Upside/Downside XLSX, checked
data and report. It does not claim forecasting, optimization or financial advice.
[Task0075](attributed-brief-action-plan.md) adds the deterministic D3 publication
path: every finding and action retains literal evidence from its exact frozen
source, questions remain explicit and absent owners/dates stay null. It does not
claim semantic truth, prioritization or staffing quality.
Finish the remaining per-ability quality, licensing/distribution and platform
release matrix on this foundation; broad catalog entries still do not count as
completed ability acceptance.
No dated estimate is committed. Preserve breadth rather than moving areas
beyond 1.0.0.

Store each ability's commit, exact tool versions, test commands, verified output,
revision proof and limits as it becomes implemented. Keep unmeasured entries
unverified; do not count a skill catalog entry as an implemented ability.

Create-specific checks, native evidence and remaining limits are recorded in its
state file. No global skills were installed, API keys created, push, deployment
or publication performed.
