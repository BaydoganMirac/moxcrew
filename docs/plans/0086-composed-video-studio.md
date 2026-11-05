# Task0086: Composed video studio

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card closes the deterministic V4 composition gap
without a live provider/model call, account action, dependency installation or
publication.

## Problem

Moxcrew can cut and reorder footage, render selected ranges, burn captions and
export three aspect variants. It cannot yet place an exact title, image and
approved audio resource into one checked playable composition. A text request
or content sidecar is not proof that those elements appear in the video.

## Scope

1. Add `video.compose` for one exact source video, one exact overlay image, one
   exact audio resource, a title, bounded placement times, an enumerated corner,
   and explicit mix or replace audio behavior. Preserve all inputs unchanged.
2. Normalize the source to 1280×720 MPEG-4, render the title visibly, scale and
   place the image only for the requested interval, and mix or replace audio at
   the requested bounded level.
3. Probe every input and the result, reject missing streams or invalid ranges,
   decode the complete output, verify duration/dimensions/audio and retain
   start, middle, overlay and end frames.
4. Emit a composition manifest with exact source hashes, placement/audio facts,
   output stream facts and verification booleans.
5. Add a localized native Project-tool form that selects the exact video, image
   and audio resources, makes timing/placement/audio policy visible, and stages
   the typed request in Chat without starting work.

## Non-goals

- No content-based clip selection, generative motion, speech generation, voice
  cloning, transcription, publication or automatic aesthetic-quality claim.
- No source mutation and no unbounded arbitrary FFmpeg expression or filter
  input from the owner.
- No live provider/model/account call, dependency installation or publication.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Deterministic tests reject missing video/image/audio streams, invalid time
  ranges, unknown positions and unsafe argument shapes before publication.
- Real local video, image and audio inputs produce a playable 1280×720 MP4 with
  visible timed title/image placement, checked audio, full decode and retained
  representative frames.
- The minimum native form is scroll-reachable in Turkish and English, both
  themes, freezes exact resource IDs and emits no Host command.
- Full tests, strict Clippy, formatting, checksums and diff checks pass.
