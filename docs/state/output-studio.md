# Output Studio and local production

Updated: 2026-11-04. Task0044 baseline; later extensions linked below.

Task0052 extends this baseline with real Office/PDF page rendering, native
region/time selection, typed image generation, OCR/transcription, video packages,
static browser work and offline interactive outputs. Its current implementation,
engine limits and verification are in [eleven capabilities](eleven-product-capabilities.md).

The native Project Outputs view opens Project-scoped Output Studio. It lists actual
files, SHA-256 versions and revision parents. A selected revision freezes the
source bytes and selected section into a new queued job; starting it remains
explicit. Retrying a failed revision validates the same frozen manifest and
preserves the selected provider. Hash-pinned export refuses changed source bytes.
Original outputs are retained. Images render natively; PPTX/DOCX previews expose
their real text structure, and media exposes checked metadata and a thumbnail.
This is not a full Office renderer, video editor timeline or inline media player.

Task0065 adds a verification summary before the selected preview. It appears
only when Project, work item, path and SHA-256 match the exact selected artifact.
The card reports exact-version proof separately from format inspection: readable
text was reopened, Office sections were parsed, image bytes were loaded or media
streams and duration were parsed. Unsupported or malformed previews remain
inspection-limited. Every state names the human check still required for content
accuracy, completeness, Office layout, image fidelity or full media playback.
See [artifact verification](artifact-verification-summary.md).

Task0066 turns the existing `parentVersionId` into an explicit local review
flow. If the exact parent hash remains in the selected Project's current Studio
listing, **Compare with previous version** reads that
file once, restores the selected current exact preview and presents bounded text
or Office structure together. The ephemeral cache holds at most eight scoped
comparison excerpts and clears on Project or Host-epoch changes. The view does
not compare image pixels, play media, infer meaning, merge versions or accept an
output. See [revision comparison](exact-revision-comparison.md).

Task0067 adds a durable owner review decision for one exact Project/work/path/
SHA-256 tuple. The Host rereads current bytes before recording `reviewed`,
`needs_changes` or a clearing `unreviewed` transition. Output Studio displays
only the matching version's active decision; a later hash never inherits it.
This is human workflow evidence, not semantic, factual, visual or playback
verification. See [artifact owner review](exact-artifact-owner-review.md).

Task0047 adds name/path search, file-type filtering and inspectable multi-file
selection. **Create package** opens a dedicated selection view; **Export package**
saves one standard ZIP and provenance manifest using checked original bytes.
Limits are 32 files, 25 MiB per file and 64 MiB total. Search does not discard
hidden selections, and stale versions require review rather than silent removal.
See [delivery packages](delivery-packages.md) for the native flow and verification.

Task0068 carries Task0067's exact owner decision into that delivery flow. File
rows, the selected-files menu and footer show review coverage; unreviewed and
needs-changes selections produce visible non-blocking warnings. The ZIP schema2
manifest records the status for the exact exported hash without changing the
delivery receipt. See [review-aware delivery](review-aware-delivery.md).

Owner-imported resources are frozen in private per-Project storage. Active agents
can list/read only their bound Project's resources. Binary material is consumed
through typed production inputs; resources.extract handles Office/PDF text.
Delivery references from another same-Project crew are read only after checked
terminal delivery, with sender/recipient and source-hash verification.

Thirteen scoped MCP tools now cover accepted decisions, crew communication,
offers/review, resources and production. production_run returns a durable job ID
quickly; production_read reports completion and actual file hashes. At most four
production children run concurrently and each agent run can start at most 32.
Stable nonces deduplicate requests. A run ending revokes its jobs. Restart records
interruption; it does not silently rerun production. A shared bounded child-process
runner handles deadline, cancellation, inherited pipes and owned descendants.

The bundled Python worker uses only the standard library. It supports editable
PPTX creation/selected-slide revision, DOCX creation/selected-paragraph revision,
CSV trimming/deduplication/numeric statistics, literal-cell XLSX, checked CSV
sum/average charts with static SVG/data/report outputs, formula-backed three-case
scenario workbooks with checked totals, image resizing,
literal-evidence structured tables with visible uncertain/missing cells,
editable attributed briefs with explicit questions and action assignments,
explicit DOCX template filling with visible missing values,
2–12 image batch conversion with per-source results, video clip concatenation
with an optional SRT sidecar, audio cleanup, and 1–12 segment podcast packaging
to checked M4A plus output-relative chapter JSON. Python
3.9+ is required; FFmpeg/FFprobe and optional pdftotext are detected/configured
separately. No binary/model is installed or bundled by these operations.
See [batch image conversion](batch-image-conversion.md) for its exact limits and
real-FFmpeg evidence.
See [podcast editing](podcast-editing.md) for its source/range contract and
real-FFmpeg/FFprobe evidence.
See [checked CSV charts](checked-csv-charts.md) for its aggregation, escaping and
static-output contract.
See [editable scenario workbooks](editable-scenario-workbooks.md) for its exact
column, fixed-formula and independent recalculation contract.
See [traceable structured extraction](traceable-structured-data-extraction.md)
for its source, literal-evidence and missing/uncertain-cell contract.
See [attributed brief and action plan](attributed-brief-action-plan.md) for its
finding/action evidence and explicit question/assignment contract.
See [DOCX template filling](explicit-docx-template-filling.md) for its exact
placeholder set, split-run handling and visible missing-value contract.

Limits: 25 MiB total input/output per production job, 12 sources, 16 produced
files, 120 seconds per job, 20 minutes maximum media input/edit duration, bounded
Office archives and CSV cells. Video clips must have consistent audio presence;
unsupported combinations fail explicitly. Caption files are sidecars. The audio
operation produces mono 24 kHz WAV. Media demuxers are forced to supported single
file formats, with network protocols and external movie references disabled.
Preview reads have a separate 12-second deadline and do not hold the command lock.
Studio listing hashes at most 64 MiB per refresh; further outputs remain available
through the existing per-job file panel.

V12 events record resources, revision provenance, production outcomes and explicit
app-action dispatch. Secrets never enter them. V13 usage records contain only
provider-reported token/cost values; missing measurements remain null. Existing
Cloud allowlists remain unchanged: V13 Claude/Gemini work is omitted from the
legacy Codex remote-control projection, and all content remains local by default.
V15 local review events add only the exact artifact scope and owner decision.
V16 adds the owner's bounded local note while retaining V15 replay; notes do not
enter delivery manifests. Neither version adds file content or widens Cloud
allowlists. See [exact owner review notes](exact-owner-review-notes.md).

Resources, preview/export and production publication use checked Unix file
descriptors. Windows remains explicitly unsupported for these file operations
until pinned no-reparse handles are implemented and verified; no unsafe fallback
is enabled. Other native platform/package release checks remain separate.

Verification evidence is recorded in the [Task0044 report](productive-project-crews.md). Live model inference,
real app credentials, external writes and public release signing are not part of
the local fixture proof.
