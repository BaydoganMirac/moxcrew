# Task0091: Verified local transcription

Status: Complete bounded path; local engine quality proof pending

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card strengthens the existing local A1 adapter
without a live provider/model call, account action, dependency installation or
publication.

## Problem

The bundled worker can call an explicitly configured whisper.cpp executable,
but the native Create outputs form treats transcription as a generic operation.
The output also lacks source/output hashes, normalized-audio verification, VTT
delivery and an explicit separation between pipeline checks and transcript
accuracy review.

## Scope

1. Make the existing transcription choice accept only exact audio/video Project
   sources and expose a bounded automatic-or-ISO language choice.
2. Preserve the exact source hash and inspect its audio stream before invoking
   the configured local whisper.cpp model.
3. Normalize to checked 16 kHz mono PCM, fully decode it and require duration
   preservation before accepting engine output.
4. Emit transcript text/JSON, SRT, VTT and a manifest with hashes, timeline and
   verification facts.
5. Reopen generated subtitle files and retain a real-media/protocol-peer fixture
   plus a native Turkish dark capture.

## Non-goals

- No bundled Whisper binary or model, dependency installation or model download.
- No live provider/model/account call and no speech generation or translation.
- No automatic claim that fixture text, punctuation, language detection,
  speaker identity or uncertain passages are semantically correct.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Invalid language/source/timeline data fails without publishing a result.
- Tests prove exact source/language freezing, no direct Host action and form
  reachability at 800×560 in Turkish and English and both themes.
- Retained evidence proves real FFmpeg/FFprobe media normalization and decode,
  deterministic whisper protocol handling, subtitle reopen and pinned hashes.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.

## Outcome

The owner-visible exact-source form, normalized-audio checks, SRT/VTT delivery,
hash manifest, retained protocol-peer evidence and native capture are complete.
The full workspace passed 593 tests with 14 explicit ignores and no failures;
strict Clippy, formatting, Python syntax and diff checks also pass. Real-model
recognition quality remains explicitly pending. See
[verified local transcription](../state/verified-local-transcription.md).
