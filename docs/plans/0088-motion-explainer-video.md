# Task0088: Motion explainer video

Status: Complete

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card closes the deterministic V5 rendering gap
without a live provider/model call, account action, dependency installation or
publication.

## Problem

V5 promises a promo or explainer video from a brief, but the product has no
typed renderer that turns an approved scene plan into real motion. A storyboard
or prose response is not a video, and a static slideshow is not motion proof.

## Scope

1. Add `video.motion` for an exact title, two to eight approved text scenes,
   bounded durations, one enumerated motion treatment and 24/25/30 fps.
2. Reuse the existing editable Mox presentation layouts and checked
   LibreOffice/Poppler rendering path to create the exact storyboard frames.
3. Encode every scene with real pan/zoom movement, bounded fades and silent AAC
   audio, then concatenate to a new 1280x720 MPEG-4 video.
4. Probe and fully decode the result, prove intra-scene frame changes away from
   fades, retain start/middle/end frames and pin storyboard/output hashes in a
   manifest.
5. Add a localized native form that freezes the exact scene plan in Chat and
   never starts production directly.

## Non-goals

- No claim that generated copy, pacing or aesthetics are automatically good.
- No text-to-video model, stock-footage retrieval, speech generation, voice
  cloning, face animation, arbitrary FFmpeg expression, publication or source
  mutation.
- No live provider/model/account call or dependency installation.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Deterministic tests reject unknown motion and invalid scene shapes before any
  external renderer runs; desktop tests prove exact scene freezing and no Host
  action.
- A real local render produces an editable storyboard and playable motion MP4,
  full-decode evidence, distinct intra-scene motion hashes and retained frames.
- The form stays scroll-reachable at 800x560 in Turkish and English and both
  themes.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.

## Verification

- Focused core, desktop and real LibreOffice/Poppler/FFmpeg render checks pass.
- The sequential all-target/all-feature workspace run passed 586 tests with 11
  explicit real-tool, PostgreSQL or manual fixture ignores and no failures.
- Strict all-target/all-feature Clippy with warnings denied, formatting, Python
  syntax and diff checks pass.
- The retained editable storyboard, fully decoded motion MP4, per-scene motion
  hashes, start/middle/end frames and real Turkish dark native form capture are
  recorded in [Task0088 state](../state/motion-explainer-video.md).
