# Task0052: Eleven conversation-first product capabilities

Status: Owner-approved, implementing

Updated: 2026-10-20

Authorization: “12. Adım hariç hepsini yapıyoruz o zaman planla ve yap! Bitmeden
gelme.” This approves all eleven proposed capabilities and excludes phone/Cloud
task intake. Implement and validate the complete local scope without intermediate
scope approval. Preserve the automatic, optionally overridden provider choice.

## Acceptance ledger

1. **Rich conversation input:** native file drop/pick for multiple resources,
   image/screenshot annotation and audio/video input. Frozen Project resources
   are available to the planner/agents. Executable OCR/transcription adapters
   expose real text/timing or an actionable missing-engine diagnosis; no invented
   transcript. Microphone/screen capture, if provided, starts only by user action.
2. **Visual targeted revisions:** real Office page/slide previews when a renderer
   is available, native image region selection and scrub/select video ranges.
   Chat instructions freeze the selected artifact hash/section/range; originals
   survive. Missing renderers retain structural previews and truthful setup.
3. **Image generation/editing:** real typed raster generation/edit jobs through a
   configured local image engine, with bounded requests, cancellation, checked
   output and private configuration. Include reference/mask edits. A resize or
   handcrafted placeholder does not count as generation. Verify protocol failures
   with a real-file peer; do not claim live model quality without a live engine.
4. **Video content packages:** transcript/timing input, selected highlights,
   portrait/square/landscape exports, burned captions, covers and associated text
   in a checked delivery. Verify actual FFmpeg output, dimensions and timestamps.
5. **Brand/style profile:** durable Project-scoped logo/resource references,
   colours, tone and style instructions, readable by bound agents and used by
   subsequent work. Edit/revision semantics; no global authoritative decisions.
6. **Browser work:** isolated owner-installed browser, fixed typed inspection,
   navigation, source capture and form drafts. No arbitrary JavaScript/shell tool,
   existing browser profile, embedded engine or implicit submission. Public URL
   validation, bounded files, private-network/redirect controls and owned-process
   cleanup. Verify actual browser rendering and form-draft output.
7. **Teach a reusable skill:** recordings/resources can be extracted by connected
   agents into an inspectable proposed step list, sources and expected outputs.
   Owner accepts/revises a Project skill before it becomes standing instructions.
   Reuse accepted skills through scoped context; retain recording/source hashes.
8. **Event-triggered work:** durable, explicitly armed folder/URL-change triggers
   bound to a frozen recipe/Project and budgets. Stable occurrence identity,
   debounce, pause/resume, restart recovery, no duplicate/late-run storm, and no
   automatic external write authorization. Host must be running.
9. **Parallel alternatives:** create/track two to four same-Project variants from
   a frozen crew/goal, explicit start and independent scopes; compare deliveries
   and select files from variants for a new follow-up without mutating originals.
10. **Inspectable evidence:** claim/source records bind exact frozen resource
    hashes, literal quotations/locations and observation dates. Clicking evidence
    reveals its actual source excerpt, including disputed evidence. Agent labels
    do not constitute an independently verified truth score.
11. **Interactive outputs:** actual standalone offline HTML tools (filterable
    tables/charts, budget controls/comparisons), safely generated from typed data,
    opened/exported deliberately in the system browser. No arbitrary injected
    script, external networking or embedded application browser engine.

## Architecture and ownership

See ADR0050. Preserve native Rust/egui and one canonical writer. Existing output,
resource, production, workflow and provider adapters win over new frameworks.
V14 local capability facts may add profiles, skills, triggers, alternatives and
evidence; V1–V13 retain exact meanings. New full text and resource paths stay out
of default Cloud payloads. Ordinary snapshots contain metadata only; explicit
Project reads supply full local data. Existing private tool configuration stores
keep credentials out of events and logs.

- Coordinator: shared Host/domain/store/MCP wiring, browser/public-fetch modules,
  UI bridge/root/i18n/native path pickers, ADR/product/state and final integration.
- Production builder: production.rs, bundled production tools and operation tests;
  image engine, OCR/transcription/render adapters, video package, interactive HTML.
- Capability builder: new project_capabilities.rs/tests and project_workflows.rs;
  durable profile/skill/trigger/variant/evidence contracts and scoped operations.
- UI builder: chat.rs/create.rs/conversation.rs/crew.rs/output_studio.rs and a new
  capabilities UI module; rich intake and native visible use of the eleven paths.
  Shared Host contracts, bridge/root/i18n and schema remain coordinator-owned.
- Independent review: stable integrated artifacts after a builder finishes;
  disjoint ownership, no duplicate tests/probes/captures or publication.

## Completion gates

Each ledger row needs implementation, user entry, real operation evidence,
revision/failure checks and documented engine/platform limits. An unavailable
optional engine must offer configuration rather than claim completed work. No
fake success, transcript, generated image, citation or scheduled occurrence.

Run focused boundaries, full workspace/all-feature tests, strict Clippy and fmt;
real Office/media/HTML/browser output checks, replay/idempotency/Cloud canaries,
native TR/EN dark/light and minimum-window interactions/captures. Independently
review and package a feature-off macOS development artifact. No remote task intake,
Git push, Cloud deployment, public release or installation into user applications.
Use synthetic inputs; never inspect private credentials or browser profiles.
