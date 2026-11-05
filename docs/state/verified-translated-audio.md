# Verified translated audio

Updated: 2026-11-05

Status: Complete bounded A5 adapter and native preparation path; real voice and listening review pending

Plan: [Task0098](../plans/0098-verified-translated-audio.md)

**Project tools → Create outputs → Create translated audio** stages one exact
Project recording, source transcript, owner-approved translation and locale pair
in Chat without starting production. The form requires an explicit confirmation
that the owner reviewed the source/translation meaning pair. Moxcrew does not
translate or transcribe the text and does not treat that confirmation as an
automatic semantic verdict.

The local `audio.dub` worker accepts one WAV, MP3 or M4A with one audio stream,
checks a duration between 0.25 and 240 seconds and hashes the frozen source. It
synthesizes only the exact approved translation with owner-configured Piper and
a target-locale ONNX/config voice pair. If the raw narration would require a pace
outside 0.5×–2×, production fails instead of forcing an unusable result.

Within that bound, FFmpeg time-fits the narration to the source duration and
performs measured two-pass normalization. `translated-audio.wav` must reopen as
one 48 kHz mono PCM stream, match the source within 50 ms or 0.2%, decode fully
and finish within one LU of -18 LUFS. `translation-content.json` retains both
exact texts; `translation-manifest.json` pins source, text, model, configuration
and output hashes plus pace/duration evidence; `summary.md` separates performed
checks from remaining human review. Absolute tool/model paths and model filenames
do not enter the deliverables.

## Verification

- The deterministic core protocol peer exercises the real Piper argument shape,
  exact private translated-text input, source/raw/output probes, pace filter,
  duration match, loudness passes and full FFmpeg decode.
- Core coverage proves the four-file result, paired exact content, source and
  manifest hashes, no private model-path leakage, required meaning review and
  rejection of a pace outside the supported range.
- Native coverage proves that unreviewed meaning cannot stage a request, then
  freezes the exact source, text pair, locales and review flag without issuing a
  Host production command. The action is scroll-reachable at 800×560 in both
  locales and themes.
- A5 now uses the same honest Python/FFmpeg/FFprobe/Piper/model readiness as A2;
  no remote voice-access claim or automatic installation remains in its profile.
- The retained Turkish dark-theme native screenshot is
  `docs/screenshots/translated-audio/translated-audio-form-tr-dark.png`:
  1600×1120, 286,383 bytes, SHA-256
  `a1ac729776cfcfbc1a3362f59edfa6ccadefffe5fb6a769243553db7dd497957`.
- Python 3.9 syntax compilation, `cargo fmt --all -- --check`, full-target/
  all-feature workspace checking and strict Clippy all pass.
- The sequential all-target/all-feature Rust gate passes 610 tests with zero
  failures; 16 explicitly environment/evidence-gated tests remain ignored. The
  final clean Node 24 legacy-reference run passes 153/153 tests.
- No real voice model was downloaded or invoked.

## Limits

This closes a bounded owner-visible A5 production path, not automatic
translation or full dubbing. The operation uses one whole-recording timing target;
it does not align sentences, preserve pauses, perform lip sync or verify that the
source transcript matches spoken audio. The owner must listen for pronunciation,
naturalness, timing suitability and accessibility, and review the selected voice
model's licence. Voice cloning, model discovery/download, remote generation,
publication, Windows-native behavior, signing and public release remain separate
gates.
