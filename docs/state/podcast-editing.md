# Podcast editing package

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0071](../plans/0071-podcast-editing.md)

**Project tools → Create outputs → Edit podcast** now accepts an episode title,
one available Project audio/video resource and 1–12 explicit segment rows. Each
row is start | end | chapter title. The visible order is the edit order; the form
stages an exact typed request in Chat and does not send, prepare or start it.

The local audio.podcast worker checks every frozen source with FFprobe, rejects
missing audio and out-of-range edits, renders each requested interval to a
consistent private PCM segment, then concatenates and encodes a static
podcast.m4a. It strips inherited metadata and verifies that the output is audio
only and within the requested duration tolerance.

chapters.json contains the episode title and recomputed output-relative chapter
times. It does not contain owner paths or private scratch references.
validation.json records bounded FFprobe format/stream evidence. Invalid input
fails the atomic operation rather than publishing a partial episode.

The operation retains the existing 12-source, 25 MiB input/output, 16-output,
120-second process and 20-minute edited-duration bounds. It installs nothing and
does not call a model, provider, account, app or Cloud service.

## Verification

- Deterministic focused coverage proves two ordered segments, output-relative
  chapter timing, an actual M4A signature, range denial and path redaction.
- The same focused test passes installed /opt/homebrew/bin/ffmpeg and
  /opt/homebrew/bin/ffprobe with a real two-second PCM WAV input.
- The desktop test passes at 800×560 and proves exact resource/segment staging,
  exclusion of an image resource and zero direct commands.
- Retained native evidence: [Turkish dark podcast form](../screenshots/podcast-editing/podcast-editing-tr-dark.png),
  SHA-256 9f23e613665688eb26ea3dda4631a9f0c58dc780d67071a40efed6d124efaf87.
- The full workspace/all-feature suite passes **545 tests** with **6 explicit
  fixture/manual probes ignored**. The core unit suite passes 111 tests with
  3 ignored, and the desktop suite passes all 248 tests.
- Strict workspace/all-target/all-feature Clippy, formatting and diff checks
  pass.

## Limits

This is deterministic cut/concatenate packaging. It does not claim content
quality, loudness correction, transcript accuracy, automatic chapter discovery
or publication readiness. Windows result access remains deferred to Windows-PC
verification.
