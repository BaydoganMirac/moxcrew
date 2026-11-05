# Verified local transcription

Updated: 2026-11-05

Status: Complete bounded path; local engine quality proof pending

Plan: [Task0091](../plans/0091-verified-local-transcription.md)

**Project tools → Create outputs → Transcribe audio** now stages one exact
Project audio/video resource in Chat without starting production. The owner
chooses automatic language detection or a bounded lowercase ISO 639 code. The
native form rejects non-media resources, preserves the exact source/language in
the typed request and shows missing whisper.cpp/model setup before Start.

The local `audio.transcribe` worker requires an explicitly configured
whisper.cpp executable and model. It hashes and probes the source, normalizes the
complete audio to 16 kHz mono PCM, checks duration and fully decodes that
normalized stream before accepting bounded engine JSON. It rejects overlapping,
out-of-range, empty or malformed segments, then emits transcript text/JSON, SRT,
VTT and a manifest. The manifest pins all deliverable hashes while keeping
content-accuracy and uncertain-passage review false.

## Verification

- Deterministic core coverage exercises the real whisper.cpp command contract,
  rejects invalid language and overlapping engine timelines, strips native
  model paths and reopens both subtitle formats. Desktop coverage proves exact
  source/language freezing, no direct Host command and action reachability at
  800×560 in Turkish/English and both themes.
- The retained [source-fixture.wav](../screenshots/transcription/source-fixture.wav)
  is a 12-second synthetic stereo tone/noise signal at 48 kHz. It is 2,304,078
  bytes with SHA-256
  `c64f702097fd529e4f46048b4792dc3eefb3acada08244d5a5134fc173fe01ea`.
  Real FFmpeg normalized the complete source to 16 kHz mono PCM; FFprobe measured
  the same 12-second duration and FFmpeg decoded the normalized stream fully.
- A deterministic local Whisper protocol peer returned two ordered fixture
  segments so the adapter could be verified without downloading or invoking a
  speech model. [transcript.json](../screenshots/transcription/transcript.json)
  is 444 bytes with SHA-256
  `cf110c017bf29c8230d348fefb63044f05bdd0b108c2e05b9c51e59bb65fefc5`;
  [transcript.txt](../screenshots/transcription/transcript.txt) is 59 bytes with
  SHA-256
  `464a0300cfd1ffde27ec1f0628ef676a49aed550a7b821f190a8f324f6b0bc58`.
  The fixture text is protocol evidence only and is not a claim about the tone
  source's semantic content or real recognition accuracy.
- [captions.srt](../screenshots/transcription/captions.srt) is 125 bytes with
  SHA-256
  `98d7c6ded1acbb719239d16d6eb2caeb4c009f1a50e4ae11b4143f650fded4f1`;
  [captions.vtt](../screenshots/transcription/captions.vtt) is 133 bytes with
  SHA-256
  `8d3dfed4b38eeb39a530f411177beca5242c422efdf97177776823a8bf0a4335`.
  Both files were reopened byte-for-byte after their non-overlapping 0–1.6 and
  1.6–3.2 second timelines were generated.
- The retained
  [transcript-manifest.json](../screenshots/transcription/transcript-manifest.json)
  is 1,215 bytes with SHA-256
  `0d0b70408a79c68ab07956b66e45b87331785e5fada8a63d7e59140ef3961734`.
  It records source/normalized-audio facts, four deliverable hashes and explicit
  false flags for content-accuracy and uncertain-passage review.
- The real 2560×1760 Turkish dark native capture
  [transcription-studio-tr-dark.png](../screenshots/transcription/transcription-studio-tr-dark.png)
  is 504,012 bytes with SHA-256
  `e63ae9bb0c35c23358740a0c135fa97c4712bfbb3817733390605338866fb03f`.
  It shows the exact recording, language policy, output set, staging action and
  honest missing whisper.cpp/model setup without clipping or horizontal overflow.
- The retained run used FFmpeg/FFprobe 9.0.2 and Python 3.9.6. Focused adapter,
  desktop, retained-output and native-smoke checks pass. The sequential
  all-target/all-feature workspace run passed 593 tests with 14 explicit
  real-tool, PostgreSQL or manual fixture ignores and no failures. Strict
  all-target/all-feature Clippy with warnings denied, formatting, Python syntax
  and diff checks pass.

## Limits

No Whisper binary or model is bundled, installed or downloaded. The retained
fixture proves media normalization, command/JSON handling, timing, file formats
and evidence integrity—not word accuracy, punctuation, language detection,
speaker identity or uncertainty marking. Those require a configured real model,
representative recordings and human review. Speech generation, translated
narration, publication, Windows-native behavior, signing and public release
remain separate gates.
