# Productive Project crews

Updated: 2026-10-19. Task0044, version 0.12.0.

Status: Complete; independently reviewed, integrated tests and macOS development package verified.

## Delivered scope

| Approved feature | Implemented behavior and evidence |
|---|---|
| Model planner | Selected provider produces validated questions or an editable roster, dependencies and acceptance conditions. Tool-free inference is explicit and cancellable. Native UI exposes the actual result. |
| Output Studio | Project-scoped hashes, provenance, native image/text/Office-outline/media previews, selected-section revision, frozen retry inputs and hash-pinned export. Originals remain intact. |
| Live direction | Owner feedback cancels/replaces affected work with revision checks, retained outputs and bounded follow-up attempts. |
| Mixed providers | Per-agent Codex/Claude/Gemini execution uses the same prepare/start/cancel lifecycle. Native protocol peers prove actual output, failed revision retry and recovery. |
| Production tools | Actual editable PPTX/DOCX, CSV/statistics/XLSX, checked charts and formula-backed scenario workbooks, image resizing/batches, video cuts/SRT, audio cleanup and podcast chapter packaging. Standard-library Python and typed FFmpeg/FFprobe operations produce checked files. |
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

`cargo test --workspace --all-features --offline --locked -- --test-threads=1`
passes: **406 tests, zero failures, five opt-in tests skipped**. The skipped
network-playlist test was run separately with explicit media tools and passed.
The other opt-in cases require private PostgreSQL fixtures, an isolated installed
Codex wrapper or the manual large-log timing probe. Final desktop-only verification
also passes all 151 tests. Strict workspace/all-target/all-feature Clippy and
`cargo fmt --all --check` pass.

The tests cover protocol-peer lifecycle, production cancellation, immutable
revisions, exact recovery, cross-Project denial, uncertain remote-write recovery,
secret canaries and old projection parity. A real HTTP MCP test produces and
hash-checks a DOCX; delivered and existing frozen inputs with colliding names
retain their distinct bytes through actual extraction.

Native fixture verification covers 43 captures: ten scenes in TR/EN and both
themes at the 800x560 minimum, plus three normal windows. Nine final app/prepared
action captures verify the readable frozen fields and friendly provider labels.
Fixture windows make no provider/account call. Actual-click UI tests verify
Prepare, Schedule and explicit action approval at the minimum size.

Real output proof uses system Python 3.9 and FFmpeg/FFprobe 7.1.1. LibreOffice
opens generated PPTX, DOCX and XLSX files. Synthetic video/audio tests exercise
real clip edits, captions, metadata, thumbnail and cleanup; a targeted slide
revision preserves other slides. A disguised HLS playlist with an MP4 extension
is rejected without contacting a loopback server. The latter is an opt-in test
requiring explicit media-tool executable paths.

The real-file proof produced an editable 2,268-byte PPTX, 908-byte DOCX,
1,447-byte XLSX, 281,174-byte edited MP4 and 96,334-byte cleaned WAV. The video
probe confirms 640x360 output and 1.021333 seconds; a real 185,528-byte PNG
thumbnail and a 9,554-byte resized PNG were also read back. These are disposable
fixtures, not a claim about model-generated design quality.

Independent integration review closed issues in V11 message allocation, timezone
handling, replay authorization for schedules and correction rounds, edited plan
fields, native provider retries, usage ingestion, V12 receipt recovery and
colliding production source aliases. The fixes have focused regression evidence.
Shared integration is in `e44a1df`; workflow review fixes are in `f5c70a7`.

## macOS development package

The release build at `20728e8ee6cb4994dc1bd716d8df3982252cd2f3` excludes
`ui-smoke`. `dist/native/productive-crews/` contains Moxcrew.app, the DMG and
`qa.json`. The app's identifier/version and required license resources were
verified, as were the ad-hoc signature and absence of smoke entry points. A real
NSWorkspace launch with disposable data and unavailable-provider stubs opened a
native window, closed normally and released the writer lock. No inference or
existing user data was used. `hdiutil verify` passes for the DMG.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| App executable | 16,406,768 | `ef9d8fd561f2123ebd7748e47eff58137c1936f8a986d0e776463c1194a46667` |
| Moxcrew_0.12.0_productive-crews_aarch64.dmg | 8,442,213 | `9a026964a614f4177be9d3117aed1410343df066493245d56bb6a6fa5af75161` |

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
