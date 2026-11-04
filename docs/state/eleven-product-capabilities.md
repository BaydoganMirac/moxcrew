# Eleven product capabilities

Updated: 2026-11-04

Status: Complete; independently reviewed, final integrated tests and macOS development package verified.

Plan: [Task0052](../plans/0052-eleven-product-capabilities.md).
Decision: [ADR0050](../decisions/0050-local-product-capabilities.md).

The approved scope is the eleven local product capabilities. Phone/remote task
intake is excluded. Completion requires the plan's implementation, visible entry,
real output/failure, replay/privacy, native review and package gates for every row.

| Capability | State | Evidence |
|---|---|---|
| Rich input | Implemented | Exact ordered multi-file import, private immutable copies, attachment manifest bound to the request even with over 32 old resources; CLI-peer OCR/timed transcription |
| Visual revisions | Implemented | Actual LibreOffice/Poppler pages, native image regions and canonical millisecond video ranges; frozen source hash/selection and original preservation |
| Image generation/edit | Implemented; local engine required | Fixed ComfyUI generation/reference/mask graph, bounded 2–12 source FFmpeg batch conversion with per-file results, real PNG protocol peer, metadata removal, scoped cancellation/abort and canonical Project configuration |
| Video content package | Implemented; real files checked | Three actual 1.6s MP4 exports, covers/content/SRT, caption pixels present only during the requested interval |
| Brand/style profile | Implemented | Project-local logo hashes/colours/tone/style; revision checks, accepted guidance in planner/run context, empty-field clearing |
| Browser work | Implemented; actual macOS Chrome proof | Actual public-page source/screenshot and frozen form draft, disabled page scripts, selected original debugger session, no submission or remote requests from frozen input |
| Taught skills | Implemented | Proposed steps/sources/expected outputs, explicit accept/edit/revoke and accepted-only standing context; actual MCP authorization and nonce tests |
| Event triggers | Implemented | Real immediate-folder fingerprinting, read-only URL fixture, bounded arming/debounce/expiry/count, original-arming automatic start and restart recovery |
| Parallel alternatives | Implemented | Two to four independent frozen-recipe missions, checked start/allocation recovery, mixed-output immutable sources for one new Chat request |
| Evidence | Implemented | Actual MCP extraction publish → resource hash → literal UTF-8 byte quote, disputed stance, source excerpt, replay/tamper/privacy checks |
| Interactive outputs | Implemented; real browser proof | Offline table filtering/sorting, chart bars and budget recalculation in actual Chrome; inert injection, zero remote requests and checked deliberate output opening |

## Using the capabilities

Attach/drop files in Chat; Send freezes the full ordered batch before planning.
The original visible message is retained; canonical request context identifies
the exact imported resource IDs and hashes. Up to 12 files and 25 MiB total per
request. Files remain Project resources rather than hidden message-only uploads.
Single task cannot bypass an unsent rich-input/visual selection.

In Outputs, preview an actual rendered page, image region or video range, then
use it in Chat. **Prepare revision** creates a new queued task; Start is explicit.
The native selection supports pointer and keyboard movement/resizing. Original
bytes and version provenance remain available. Select files from different crews
through the package selection and **Use selected files in Chat** to freeze up
to 12 files/25 MiB into the follow-up; this does not start a crew.

**More → Project tools** contains Brand, Project skills, Triggers, Alternatives,
Evidence, Create outputs and Browser. Typed production/browser requests are staged
into the same Chat plan/prepare/start path. A skill proposal is not standing
guidance until accepted. Source labels and agent claims are untrusted material,
never extra permissions. Evidence opens actual text; it is not a truth score.

The forty-recipe catalog now reuses this Studio readiness through
[Task0062](ability-readiness-matrix.md). It distinguishes required local tools,
optional enhancements and separately checked access before selection and links
missing setup to the existing surfaces. This does not upgrade protocol-peer or
fixture evidence into live engine/model quality.
[Task0063](start-readiness-preflight.md) reuses the same profiles for queued Chat
and Create work, prepared Crews and the Tasks queue. Missing or unread required
local setup disables the native Start action; optional tools and access-only
checks remain visible without being treated as proven blockers.

Saving a trigger leaves it paused. Arm it for 1–1440 minutes and 1–100 occurrences;
the current source becomes its baseline and only later changes create work.
Debounce is 10–3600 seconds. It runs only while the Host is awake, one active mission
per trigger; pause/expiry/re-arm cannot start a stale prepared occurrence.
Already-started work remains linked. Retired prepared missions remain inspectable
and can be explicitly started by the owner. A watched folder covers immediate,
non-hidden regular children, at most 32 files/25 MiB each/64 MiB total; no links,
recursive ingestion or automatic full-folder import. URL conditions read up to 1 MiB
from public HTTP(S), with pinned DNS and redirect checks. Arming authorizes the
frozen local recipe and budgets, not publishing, purchases or external writes.

## Engines and concrete limits

- Python 3.9+ supports the fixed local workers. No runtime dependency was added.
  Tool setup uses existing owner-selected executable paths; nothing is installed
  automatically. The default provider remains automatic with optional override.
- Office/PDF page previews require LibreOffice/Poppler; render at most 8 exact
  pages per job, page numbers 1–100. Structural previews remain available without
  rendering. External Office references/macros/unsafe archives are rejected.
- Media uses FFmpeg/FFprobe. Caption burning prefers libass. A fixed
  LibreOffice/Poppler→PNG/FFmpeg overlay fallback supports at most 20 captions,
  each 120 characters/three lines. This fallback was tested with actual pixels.
- OCR requires Tesseract. Timed transcription requires whisper.cpp and a
  compatible local model. CLI peers verify their adapters; actual OCR/speech
  engines/models were unavailable. No transcript or quality is claimed from a
  fixture. Native file/recording input is implemented; microphone/screen capture
  is not added.
- Images require a running local ComfyUI service, supported checkpoint nodes and
  per-job cancellation. Configure its literal loopback HTTP URL and checkpoint
  privately for the Project. Inputs/masks are same-size PNGs; dimensions 64–2048,
  multiples of 8. The real-file peer verifies generation/edit integration, not
  live model inference or quality. ComfyUI retains its own uploaded inputs,
  history and outputs; Moxcrew publishes checked PNGs without embedded metadata.
- Browser uses owner-installed Chrome in a disposable profile, with page scripts
  disabled and fixed read/draft expressions. Static public pages/frozen HTML are
  supported; JavaScript-only sites, accounts, arbitrary agent JS and submission
  are outside this adapter. GET/HEAD only; private networks, credentials, other
  ports/protocols and downloads are denied. Current pipe implementation requires
  POSIX. Windows is an explicit unsupported result; Linux runtime is unverified.
- Evidence requires an actual UTF-8 resource up to 64 KiB. Extract binary sources
  first and publish the actual outputs/ text through the scoped MCP tool.
- Interactive outputs use fixed offline scripts and CSP. Only exact files from
  completed typed interactive/browser jobs may open through the owner action.
  Other HTML remains exportable. The system browser is external to the native UI.

## Verification evidence

Production commit: `56222d2`. Capability implementation: `2de4daa`; recovery
fixes: `4413f3a`. Independent source reviews found and verified fixes for image
Project IDs, HTML-open provenance, trigger re-arming, committed variant receipts,
attachment request binding, receipt refresh, folder replacement, optional brand
clearing and millisecond video endpoints. Integration is committed as `07fcaae`.
The actual browser proof found a duplicate debugger session that paused navigation;
the fixed worker resumes the original auto-attached session. Independent browser
source review passes. Both frozen form drafting and public `example.com` reads
produced actual PNG/text/JSON files with the checked final worker.

Native review also found that disabled frames could consume automatic-read flags
without dispatching the reads. `216ed11` checks the enabled UI before touching
those flags. Its regression covers studio/workflow/capability reads in order,
interleaved busy frames and no repeated reads. Smoke captures now require an idle
Bridge and seed matching cache identities. The first busy-frame capture set is
retained separately as rejected evidence; it does not establish idle usability.
`4b9eaea` bounds the entire serialized standing context to 24,000 bytes, including
its index and JSON/Unicode escaping. Omitted profile text/accepted skills are
explicitly marked and available through paginated scoped reads. A 200-record
boundary test covers accepted/proposed/revoked/foreign entries and maximal
Unicode/control-character profiles. Independent core source review passes.

At final runtime commit `072f371`, the workspace/all-features suite passed
**504 tests, zero failures, six ignored** across 28 targets, including core 107 and
desktop 212. This includes the final browser debugger and busy-frame fixes plus
attachment/localization polish. The complete Create subset passed **103/103**;
focused browser policy/protocol checks passed **2/2**. The opted-in real
FFmpeg/FFprobe network-playlist rejection passed **1/1**, with no request reaching
the private loopback fixture. Strict workspace/all-targets/all-features Clippy
(`-D warnings`), workspace formatting and `git diff --check` pass. Network-bearing
tests used isolated loopback fixtures with socket access; assertions were not
disabled. Retained logs and their hashes are in sibling
`branding/moxcrew/eleven-capabilities-qa/checks/checks.json`.

The remaining five opt-in checks cover an isolated installed Codex wrapper,
a retained ZIP for external-reader inspection, two private PostgreSQL/HTTP fixtures
and the manual 52,000-event performance probe. They are not claimed by the new capability proofs.

Browser evidence is in sibling `branding/moxcrew/eleven-capabilities-qa/browser/`:
`proof.json`, `frozen-draft/` and `public-page/`. Each source body, actual screenshot
and sanitized capture has a recorded hash. Public-page inspection made one pinned
GET; frozen drafting made zero network requests and retained the unsent field.

Real synthetic media/Office/HTML evidence is in the sibling output directory
`branding/moxcrew/eleven-capabilities-qa/production/`: `proof.json`,
`caption-pixels.json`, `html-browser-proof.json`, `final-validator-proof.json`
and the checked files. UI fixtures never establish model inference or account
access. V1–V13 legacy meanings and default Cloud allowlists remain unchanged;
V14 content stays local and snapshots carry metadata only.

Native verification uses disposable fixtures, not model runs. The complete 96
TR/EN × dark/light × normal/minimum matrix at `216ed11` passed capture/hash checks
and independent visual review. It covers empty Chat, all seven Project tool views,
image/video/interactive forms, 12 expanded attachments and an actual rendered PNG
canvas. Normal captures are 2560×1760, minimum captures 1600×1120 pixels. Files,
logs, contacts and the exact binary hash are in sibling
`branding/moxcrew/eleven-capabilities-qa/native/qa.json`.

Final polish at `453265e`/`072f371` fixes the unsupported arrow, missing Turkish
labels and attachment-context priority. Conflict explanation/Clear selection now
precede the long list; the main action and conflict control stay in view while
context remains a bounded 76 px scroller. The complete Create suite passed
**103/103** after this change; the stress check covers both locales/themes and
requires the actual Clear selection rectangle inside the clip. The five affected
scenes were recaptured in all 40 combinations with final source/binary hashes in
sibling `branding/moxcrew/eleven-capabilities-qa/native-polish/qa.json`. Final
independent source and visual review passes with no remaining blockers. The other
56 base captures are unaffected by this layout/text-only polish. Screenshots
establish appearance; pointer/keyboard/receipt behavior is covered by the tests.

The feature-off macOS arm64 development package at runtime commit `072f371` is in
`dist/native/eleven-capabilities/`: `Moxcrew.app`,
`Moxcrew_0.12.0_eleven-capabilities_aarch64.dmg` and `qa.json`. Strict ad-hoc
signature verification, bundle version/identifier/license checks, absence of
smoke/gallery markers, normal isolated launch/close, writer-lock release and
`hdiutil verify` all passed. The packaged executable is 17,576,432 bytes with SHA256
`2266e369218326955e11e947e1e304a49c13df7c847d006efbbc93207d971f93`;
the 9,009,778-byte DMG has SHA256
`7bd1d318a7f7c7eafaf7e2da8a4bc650f8e0b5aec6c0b0bf1fbcd685964f30b3`.
It is a local development artifact with an ad-hoc signature, not notarized or
installed into the owner's applications. No provider inference ran in this gate.

The source remains 0.12.0; this is not a 1.0.0 public release. Stop the old Host and
preserve a complete pre-upgrade backup before V14 writes; older builds require
that unchanged copy for rollback. No phone/Cloud intake, push, deployment,
global tool installation, live account login or public release is included.
