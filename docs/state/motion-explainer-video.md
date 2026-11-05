# Motion explainer video

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0088](../plans/0088-motion-explainer-video.md)

**Project tools → Create outputs → Create motion video** now stages an exact
approved scene plan in Chat without starting production. The owner supplies two
to eight cover/statement scenes, each with an exact duration, eyebrow, title and
subtitle, then chooses calm, push or drift motion and 24, 25 or 30 fps. The form
rejects malformed or overlong plans and preserves every accepted value in the
typed request.

The local `video.motion` worker reuses the editable Mox presentation renderer,
renders every slide through LibreOffice and Poppler, and turns those exact frames
into real fixed pan/zoom motion with bounded fades. It concatenates the scenes to
a new 1280×720 MPEG-4/AAC file, probes and fully decodes the result, proves that
two interior frames differ in every scene, and retains start, middle and end
review frames. The storyboard and video hashes remain pinned in the manifest.

## Verification

- Deterministic core coverage rejects an unknown treatment before external tools
  run. Desktop coverage proves exact scene-plan freezing, no direct Host command
  and scroll reachability at 800×560 in Turkish/English and both themes.
- The real editable
  [motion-storyboard.pptx](../screenshots/motion-video/motion-storyboard.pptx) is
  9,011 bytes and SHA-256
  `71c95d25858abd9f9552d13166992cabd4886bf2e992283f546faf1298f4b397`.
  Its four requested slides were rendered successfully; the matching structured
  storyboard hash is
  `72d8f47784c47ae3cf7c4b978fe0827e30d80bd5b116d3b82b60446d0d91594a`.
- Real retained output
  [motion-video.mp4](../screenshots/motion-video/motion-video.mp4) is 3,094,111
  bytes and SHA-256
  `f4df58de88c17af063b291466471eb7cb6d7a107cf9da3fe23ada79d06b1c502`.
  FFprobe found 1280×720 MPEG-4 video at 30 fps and silent AAC stereo at 48 kHz.
  Its 12.021354-second duration matches the four three-second scenes and FFmpeg
  decoded the complete file.
- Each scene's retained 0.75/2.25-second hashes differs, proving intra-scene
  movement away from the fades. The start, scene-three midpoint and end frames
  were inspected at 1280×720. All exact text is legible, the intended Mox layout
  remains intact and no black transition frame, clipping or overflow was found.
  The conservative machine manifest keeps `visualQualityReviewed: false`; this
  state record documents the later human frame review. The manifest SHA-256 is
  `dc15d59ddbc258af4ae771ceb70691386ff16c6f25837e57a7c084dd93128da2`.
- The real 2560×1760 Turkish dark native capture
  [motion-video-studio-tr-dark.png](../screenshots/motion-video/motion-video-studio-tr-dark.png)
  is 639,139 bytes and SHA-256
  `905d453a5eac7ce0bef3402949004b04e738f6a98b4e802c99492fc36fa5f210`.
  It shows the exact four-scene plan, treatment, fps and staging action without
  horizontal overflow or clipping.
- The retained run used FFmpeg/FFprobe 9.0.2, LibreOfficeDev 26.8.0.0.alpha0,
  Poppler `pdftoppm` 26.05.0 and Python 3.9.6. Focused deterministic, desktop and
  real-render checks pass. The sequential all-target/all-feature workspace run
  passed 586 tests with 11 explicit real-tool, PostgreSQL or manual fixture
  ignores and no failures. Formatting, Python syntax, strict all-target/all-feature
  Clippy with warnings denied and diff checks pass.

## Limits

This is deterministic motion from owner-approved text scenes, not a text-to-video
model. It does not write the brief, select footage, synthesize speech, translate,
publish or claim automatic pacing/aesthetic quality. The AAC track is deliberately
silent so the output remains playable and ready for a separately approved audio
step. Live provider content quality, Windows-native behavior, signing and public
release remain separate gates.
