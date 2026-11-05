# Task0087: Signal-based video highlights

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card closes the deterministic V2 selection gap
without a live provider/model call, account action, dependency installation or
publication.

## Problem

Moxcrew can render owner-selected cuts but cannot yet inspect a source and
propose a checked highlight reel from its actual content. Asking a model for
"the best moments" without retained measurements would make the result neither
repeatable nor reviewable.

## Scope

1. Add `video.highlights` for one exact video, a target duration, a fixed clip
   duration and an enumerated balanced, visual-motion or audio-energy emphasis.
2. Sample the complete source at bounded rates, calculate visual frame-change
   and audio RMS measurements, rank non-overlapping candidate ranges and retain
   every selected range with its component and combined scores.
3. Render the selected ranges in chronological order to a new checked 1280x720
   MPEG-4 output, decode it completely and retain one representative frame per
   selection.
4. Emit an edit plan and selection manifest that pin source/output hashes,
   sampling facts, stream facts and verification booleans.
5. Add a localized native Project-tool form that freezes the exact source and
   bounded selection policy in Chat without starting work.

## Non-goals

- No claim that motion or loudness equals semantic importance, aesthetic
  quality, factual importance or owner preference.
- No transcription, face recognition, biometric inference, generative motion,
  publication, source mutation or arbitrary FFmpeg expression input.
- No live provider/model/account call, dependency installation or publication.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Deterministic tests reject unknown emphasis and invalid duration shapes before
  publication; desktop tests prove exact resource freezing and no Host action.
- A real local source produces a playable highlight reel, exact scored ranges,
  full-decode evidence and representative frames.
- The form stays scroll-reachable at 800x560 in Turkish and English and both
  themes.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.
