# Output Studio and local production

Updated: 2026-10-19. Task0044; integration verification in progress.

The native Chat file panel opens Project-scoped Output Studio. It lists actual
files, SHA-256 versions and revision parents. A selected revision freezes the
source bytes and selected section into a new queued job; starting it remains
explicit. Retrying a failed revision validates the same frozen manifest and
preserves the selected provider. Hash-pinned export refuses changed source bytes.
Original outputs are retained. Images render natively; PPTX/DOCX previews expose
their real text structure, and media exposes checked metadata and a thumbnail.
This is not a full Office renderer, video editor timeline or inline media player.

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
CSV trimming/deduplication/numeric statistics, literal-cell XLSX, image resizing,
video clip concatenation with an optional SRT sidecar, and audio cleanup. Python
3.9+ is required; FFmpeg/FFprobe and optional pdftotext are detected/configured
separately. No binary/model is installed or bundled by these operations.

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

Resources, preview/export and production publication use checked Unix file
descriptors. Windows remains explicitly unsupported for these file operations
until pinned no-reparse handles are implemented and verified; no unsafe fallback
is enabled. Other native platform/package release checks remain separate.

Verification evidence is finalized in the Task0044 report. Live model inference,
real app credentials, external writes and public release signing are not part of
the local fixture proof.
