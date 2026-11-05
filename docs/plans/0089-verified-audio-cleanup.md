# Task0089: Verified audio cleanup

Status: Complete

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card closes the owner-visible deterministic A3 gap
without a live provider/model call, account action, dependency installation or
publication.

## Problem

`audio.cleanup` can render a WAV, but the native Create outputs studio cannot
freeze an exact cleanup policy or target level. The existing result only proves
that a stream exists and keeps no source/output hash, before/after level or full
decode evidence. That is not enough to claim the requested leveling goal was
met.

## Scope

1. Accept one exact Project audio/video resource, one enumerated cleanup preset
   and a bounded integrated-loudness target.
2. Preserve source duration and channel count, render a new 48 kHz PCM WAV and
   never mutate the original resource.
3. Measure input and output integrated loudness/true peak with FFmpeg's loudnorm
   analysis, fully decode the result and require bounded target agreement.
4. Emit a source/output-hashed manifest with stream, duration, policy and
   before/after measurement evidence.
5. Add a localized native form that freezes the exact request in Chat and never
   starts production directly.

## Non-goals

- No source separation, dereverberation, speaker repair, transcription, speech
  generation, translation, arbitrary filter expressions or publication.
- No claim that denoising improves every recording or that measured level proves
  subjective quality.
- No live provider/model/account call or dependency installation.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Invalid presets/targets fail before a renderer runs; desktop tests prove exact
  source/policy freezing and no Host action.
- A real retained noisy source produces a playable WAV whose full decode,
  source/output hashes, duration, channels and before/after loudness are checked.
- The form stays reachable at 800×560 in Turkish and English and both themes.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.

## Verification

- Focused core, desktop and real FFmpeg/FFprobe render checks pass.
- The sequential all-target/all-feature workspace run passed 589 tests with 12
  explicit real-tool, PostgreSQL or manual fixture ignores and no failures.
- Strict all-target/all-feature Clippy with warnings denied, formatting, Python
  syntax and diff checks pass.
- The retained exact source, checked WAV, source/output/measurement manifest and
  real Turkish dark native form capture are recorded in
  [Task0089 state](../state/verified-audio-cleanup.md).
