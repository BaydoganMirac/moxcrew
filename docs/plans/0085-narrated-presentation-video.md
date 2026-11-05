# Task0085: Narrated presentation video

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card closes presentation ability P5 without a live
provider/model call, account action, dependency installation or publication.

## Problem

Moxcrew can now create, revise, brand and render an editable presentation, but
it cannot turn exact speaker notes plus approved narration clips into a playable
presentation video. A generic video edit is not enough: slide order, note
presence, audio timing, source hashes and all-slide rendering need one bounded
checked contract.

## Scope

1. Add `presentations.narrate {source,segments,fps?}`. Require one ordered audio
   segment for every source slide and a non-empty speaker-note part for every
   slide. Keep the source PPTX unchanged.
2. Render every slide with the configured LibreOffice and Poppler paths, inspect
   each narration input with FFprobe, and reject non-audio, missing, duplicate,
   out-of-order, overlong or mismatched inputs before encoding.
3. Build one 1280×720 MPEG-4/AAC MP4 with fixed frame rate and exact per-slide
   audio duration. Decode the completed file, probe its streams and compare its
   duration with the checked sum before success.
4. Emit a manifest with source/audio hashes, note hashes, exact durations, slide
   order, stream facts and an explicit `spokenTextVerified: false` limitation.
5. Add a localized native Project-tool form that chooses one exact PPTX and
   1–8 exact audio resources, uses visible filename order, and stages the typed
   request in Chat without starting work.

## Non-goals

- No voice cloning, speech generation, transcript inference or automatic claim
  that spoken words match speaker notes.
- No mutation of the source PPTX and no embedding audio back into the deck.
- No live provider/model/account call, dependency installation or publication.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Deterministic tests reject missing notes, mismatched audio count/order and
  malformed arguments before encoding.
- A real noted presentation plus real local narration clips produces a playable
  1280×720 MP4 with audio, checked total duration, decoded full stream and
  retained first/middle/last frame evidence.
- The 800×560 native form is scroll-reachable in Turkish and English, both
  themes, stages exact resource IDs in filename order and emits no Host command.
- Full tests, strict Clippy, formatting, checksums and diff checks pass.
