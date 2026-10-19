# Task 0044: Productive Project crews

Status: Complete; independently reviewed, integrated tests and macOS development package verified

Updated: 2026-10-19

Approval: “Onaylı hepsini implemente et”, accepting all twelve features proposed
after Task0043. This card records that approval; implementation does not require
another product-scope confirmation. Current version remains 0.12.0.

## Observable scope

1. **Intelligent Mox:** a selected connected provider returns a validated plan,
   or explicit missing questions, from the goal and selected Project context.
   The owner can edit and prepare it; inference is visibly a provider operation.
2. **Output Studio:** Project-bound artifact previews, version/provenance, selected
   element revision, retained originals and export. Native presentation/document
   pages, images and media metadata/timeline remain beside the conversation.
3. **Live direction:** owner feedback to the mission or a selected agent affects
   bounded subsequent work; cancellation/replacement of affected running work is
   attributable, revision checked and retains earlier files.
4. **Mixed providers:** real supported Claude Code, Gemini CLI and Codex control
   behind the existing prepare/start/cancel contract, selectable per crew member.
5. **Production skills:** executable presentation/document/data/image/video/audio
   operations with typed inputs, tool preflight, checked files and useful errors.
6. **Cross-crew delegation:** same-Project task offers, acceptance, actual delivery
   references and observable completion, beyond an unstructured inbox message.
7. **Review and correction:** a reviewer reports checked conditions, accepts the
   result or requests bounded corrections; failed required checks prevent success.
8. **Project resources:** explicitly imported files are frozen with hashes and
   provenance, parsed within bounds and exposed only to the selected Project.
9. **Recurring tasks:** durable local schedules, next run, pause/resume and exact
   occurrence identity; no missed-run storm or claim that a sleeping host executes.
10. **App connections:** initial concrete operations are Google Drive file import,
    Notion page creation and Google Calendar event creation. Account credentials
    remain private; writes have a concrete preview and explicit local approval.
11. **Crew recipes:** reusable launch/content/podcast/research/travel/study/data
    compositions with roles, deliverables and tool requirements.
12. **Time and usage:** actual elapsed work and starts, limits and bottlenecks;
    provider-reported token/cost data when available, unknown otherwise.

## Architecture and boundaries

Follow [ADR0045](../decisions/0045-productive-crews-and-explicit-tools.md).
One native Host remains execution authority and canonical writer. Preserve V1–V10
event meanings and existing Cloud allowlists. V11 owns workflow facts, V12 owns
resource/artifact/action facts and V13 owns multi-provider execution facts. Legacy
fixtures must retain exact shapes and hashes when no new facts exist.

Optional external tools are owner-installed executable dependencies invoked as
bounded child processes with fixed arguments; they are not a persistent runtime
or a second authority. No shell text is accepted as a tool operation. Curated
Python scripts use the standard library for editable Office/data formats;
FFmpeg/FFprobe handle media. Rendering tools are detected separately. Never install
global software, download a model, read arbitrary credential stores or silently
change provider settings. No browser engine or Node sidecar is introduced.

File input/output is pinned, scoped and bounded; archives and external content are
untrusted data. Originals are preserved and repeated command identities return
the acknowledged outcome. Resource contents and connector credentials never
enter the default Cloud projection. Connector secrets are excluded from canonical
events and UI snapshots. Uncertain remote writes are not automatically repeated.

## Ownership

- Coordinator: shared Host/domain/store/work/creation/conversation/MCP contracts,
  new project_studio/resources/production/connections modules and curated tool
  scripts, ADRs/docs, integration and packaging.
- Provider builder: providers.rs, provider_connections.rs, optional cli_control.rs,
  provider-specific tests/state. Real CLI protocol discovery, scoped MCP adapters,
  bounded structured generation and actual usage observations.
- Workflow builder: project_crews.rs and new project_workflows.rs plus their tests
  and feature state. V11 planning/lifecycle/steering/delegation/review/recipes/
  scheduling/budgets. Root supplies V13 queue and V12 resource/tool integration.
- UI builder: chat.rs/create.rs/crew.rs/i18n.rs, bridge.rs/ui.rs/main.rs and new
  output_studio.rs plus embedded tests/state. Existing chat, crew and output
  surfaces cover all twelve features; shared Host contracts remain root-owned.
- Independent reviewer: stable integrated commits and real acceptance evidence;
  no implementation ownership, push or publication.

## Verification and handoff

Use disposable directories, supported fake CLI peers for all three providers,
real bounded subprocess output tests and loopback connector fixtures. Verify
exact receipts, crash prefixes, invalid/stale/foreign Project denial, immutable
files, cancellation, timeouts, unknown usage, repeated schedule occurrences,
uncertain remote write recovery and Cloud canaries. Generated Office/media outputs
must pass actual format/dimension/duration/open checks; recipes alone do not count.

Verify TR/EN, both themes, 800x560 and normal native windows; preserve drafts and
selection across asynchronous responses. Run formatting, focused boundaries,
workspace tests and strict Clippy; independently review and package a feature-off
macOS development artifact. Record provider/platform/tool/account limits honestly.

No live inference, provider login, API key creation, third-party account mutation,
Cloud deployment, Git push or public signing is needed to implement and validate
this card. Missing external credentials do not block local fixtures and UI work.

Final evidence and remaining platform/account/1.0.0 boundaries are recorded in
[productive Project crews state](../state/productive-project-crews.md).
