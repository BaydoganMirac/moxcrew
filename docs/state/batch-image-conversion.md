# Batch image conversion

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0070](../plans/0070-batch-image-conversion.md)

**Project tools → Create outputs → Batch images** now lets the owner choose 2–12
frozen image resources and stage one precise request in Chat. The form records a
maximum width and height, keeps aspect ratio explicit and does not send, prepare
or start work by itself.

The new typed `images.batch` operation accepts an ordered list of exact sources.
The Host applies the existing Project/run authority, copies at most 12 sources
into private scratch and remaps only checked source fields. The bundled worker
uses configured FFmpeg with pinned single-file image demuxers, converts each
source to PNG within the requested bounds and validates/rewrites the PNG without
ancillary metadata.

Each source receives an ordered `converted` or `failed` item. A malformed or
unsupported source removes only its own incomplete output; successful siblings
remain available. `batch.json` records counts, dimensions and safe reason codes
without owner paths or source bytes. A missing FFmpeg blocks the operation rather
than returning an all-failed success. Existing 25 MiB input/output, 16-file,
120-second and private-scratch limits remain authoritative.

No model, provider, account, external app or Cloud action is involved in the
operation proof. The installed `/opt/homebrew/bin/ffmpeg` converted a real PNG
while a malformed sibling produced only its per-file failure result.

## Verification

- Focused core coverage proves ordered partial success, 2–12 bounds, PNG
  validation and removal of a private metadata canary.
- Focused desktop coverage proves exact resource selection/dimensions, no direct
  command and no accidental inclusion of an unselected text resource.
- The installed `/opt/homebrew/bin/ffmpeg` passes the same focused conversion
  proof; this is not a fake-worker-only claim.
- The full workspace/all-feature suite passes **543 tests** with **6 explicit
  fixture/manual probes ignored**. The focused core suite passes 110 tests with
  3 ignored, and the desktop suite passes all 247 tests.
- Strict workspace/all-target/all-feature Clippy, formatting and diff checks
  pass.
- Retained native evidence: [Turkish dark batch form](../screenshots/batch-image-conversion/batch-image-conversion-tr-dark.png),
  SHA-256 `fc381d8066333fe98e6f3d2f4bfe0581359a007d6606b2703bd7171f4bfb6bf9`.

## Limits

Output is PNG only and uses fit-within bounds; it does not crop, upscale to an
exact canvas or claim visual quality. Windows result access remains deferred to
verified Windows-native no-reparse file operations.
