# Productive Project crews

Updated: 2026-10-19. Task0044, version 0.12.0.

Status: Implementation complete; final integrated verification in progress.

## Delivered scope

| Approved feature | Implemented behavior and evidence |
|---|---|
| Model planner | Selected provider produces validated questions or an editable roster, dependencies and acceptance conditions. Tool-free inference is explicit and cancellable. Native UI exposes the actual result. |
| Output Studio | Project-scoped hashes, provenance, native image/text/Office-outline/media previews, selected-section revision, frozen retry inputs and hash-pinned export. Originals remain intact. |
| Live direction | Owner feedback cancels/replaces affected work with revision checks, retained outputs and bounded follow-up attempts. |
| Mixed providers | Per-agent Codex/Claude/Gemini execution uses the same prepare/start/cancel lifecycle. Native protocol peers prove actual output, failed revision retry and recovery. |
| Production tools | Actual editable PPTX/DOCX, CSV/statistics/XLSX, image resizing, video cuts/SRT and audio cleanup. Standard-library Python and typed FFmpeg/FFprobe operations produce checked files. |
| Cross-crew delegation | Same-Project offers, acceptance and terminal hash-checked delivery; scoped agents can consume delivered files through production inputs. |
| Review/correction | Required checks and reviewer acceptance gate completion. Return feedback requests bounded correction work. |
| Project resources | Explicitly imported files are frozen with hashes, scoped to the Project and readable only by authorized agents; typed extraction handles Office/PDF text. |
| Recurring tasks | Durable recipes/schedules, pause/resume, occurrence identity, one active occurrence and skipped missed intervals on sleeping hosts. |
| App connections | Private per-Project manual credentials; exact preview/approval for Drive file import, Notion page creation and Calendar event creation. Connection changes invalidate old previews. |
| Crew recipes | Reusable launch/content/podcast/research/travel/study/data roles and deliverables. |
| Time/usage | Actual elapsed work, starts and provider-reported tokens/cost. Missing usage stays unknown; mission job/time/review limits are enforced. |

Detailed contracts: [workflows](workflow-controls.md),
[provider execution](provider-execution.md), [Output Studio](output-studio.md),
[app connections](app-connections.md), [native interface](desktop-productive-integration.md).

## Compatibility and privacy

V11 workflow, V12 Studio and V13 execution events extend the local append-only
store. V1–V10 meanings and legacy projection fixtures remain unchanged. Exact
legacy replay and Cloud-boundary tests cover the extension. Workflow full text,
resources, outputs, credentials and usage do not widen the existing Cloud
allowlist. New Claude/Gemini execution is omitted from the older Codex remote
projection. The private Cloud repository is unchanged.

Before opening existing data with this build, stop the Host and preserve a full
data-directory backup. Older binaries cannot read new event versions; rollback
requires restoring that stopped pre-upgrade backup. Never delete new events from
an active canonical log.

## Verification

Final workspace totals, native captures and package evidence are recorded here
after the integrated pass. Focused evidence already includes protocol-peer
lifecycle, production cancellation, immutable revisions, exact recovery,
cross-Project denial, uncertain remote-write recovery, secret canaries and old
projection parity.

Real output proof uses system Python 3.9 and FFmpeg/FFprobe 7.1.1. LibreOffice
opens generated PPTX, DOCX and XLSX files. Synthetic video/audio tests exercise
real clip edits, captions, metadata, thumbnail and cleanup; a targeted slide
revision preserves other slides. A disguised HLS playlist with an MP4 extension
is rejected without contacting a loopback server. The latter is an opt-in test
requiring explicit media-tool executable paths.

## Boundaries

- This completes Task0044's bounded feature set, not the complete forty-ability
  1.0.0 release matrix. Source version remains 0.12.0.
- Python 3.9+, FFmpeg/FFprobe and optional pdftotext must be available locally.
  No external binary, model or global skill is installed or redistributed.
- Office previews show real text structure; video previews show checked metadata
  and thumbnails. Full Office rendering and inline video playback are separate.
- Schedules execute while the Host is awake. Recurring recipes currently use
  Codex by default. App connections use manually supplied tokens, not OAuth UI.
- Real model inference, user entitlements and real account writes were not used.
  Native Windows/Linux runtime and packaging remain unverified. Pinned Studio
  file operations currently fail closed on Windows.
- Development signing is ad-hoc. Public signing/notarization, release publication,
  deployment and Git push are separate owner actions.
