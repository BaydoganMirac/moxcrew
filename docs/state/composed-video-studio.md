# Composed video studio

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0086](../plans/0086-composed-video-studio.md)

**Project tools → Create outputs → Compose video** now stages a bounded typed
request in Chat without starting production. The owner selects one exact video,
one exact overlay image and one approved audio resource, then sets the visible
title duration, image interval, one of four corners, mix or replace behavior and
the added-audio level. The form keeps every choice visible and does not accept an
arbitrary FFmpeg expression.

The local `video.compose` worker probes all three inputs, preserves their bytes
and rejects missing streams, unsupported formats, unknown positions, invalid
time ranges and a mix request whose source has no audio. It renders a branded
ink-and-orange title band, scales and places the image only during the requested
interval and loops the approved audio as needed. The result is a new 1280×720,
30 fps MPEG-4/AAC file. The worker probes its dimensions and streams, checks its
duration, decodes the complete video and retains start, middle, placement and
end frames. Its manifest pins every source and the output hash.

## Verification

- Deterministic core coverage rejects an unknown image position before tools run
  and rejects a probed source without a video stream before rendering. Desktop
  coverage proves exact resource freezing, no direct Host execution and
  reachability at 800×560 in Turkish/English and both themes.
- Real retained output
  [composed-video.mp4](../screenshots/video-composition/composed-video.mp4) is
  2,483,637 bytes and SHA-256
  `8ed572cf6457ed2e466b4ce8f454d93348fb5d9008193ed7988d9ef31abc7844`.
  FFprobe found 1280×720 MPEG-4 video at 30 fps and AAC stereo audio at 48 kHz.
  Its 15.579938-second duration matches the source, FFmpeg decoded the full file
  and audio analysis measured -25.4 dB mean and -7.5 dB maximum volume.
- The retained run uses the exact source-video hash
  `6178e65a941a66dce8f74cc2c5cc9f13ff686f58ddeb5e30547b8315ed8886d0`,
  Mox image hash
  `71188e35a3ad61573651ba48c2933336791c46f6dce244625dc7cde3a21a9149`
  and approved-audio hash
  `0c47f2046f1b29367b4a89fc5f8d8aab668d7774a3ef00cc723d63250cef2f4c`.
  Replace mode at level 0.8 produced the retained file; mix mode at level 0.25
  was run separately and also passed stream, duration and full-decode checks.
- All four 1280×720 frames in
  [`docs/screenshots/video-composition/`](../screenshots/video-composition/) were
  inspected. The start shows the requested branded title band, the placement
  frame shows the intact Mox image at bottom right during seconds 3–12, and the
  end frame shows that the timed overlay is absent. No clipping or missing source
  content was observed. The manifest is SHA-256
  `5af0661c4ad9b9601072b88f1d9710e44cf3d4a69a20a6d3954fc392d9961e4d`.
- The real 2560×1760 Turkish dark native capture
  [video-composition-studio-tr-dark.png](../screenshots/video-composition/video-composition-studio-tr-dark.png)
  is 591,125 bytes and SHA-256
  `410b9c9d65d96e4faa9af22091053ebca952c8e3fe54f2962ca3e5ecf5ed671d`.
  It shows all exact resources, timing, position, audio policy, level and the
  staging action without horizontal overflow or clipping.
- The retained run used FFmpeg/FFprobe 9.0.2, Poppler `pdftoppm` 26.05.0 and
  LibreOfficeDev 26.8.0.0.alpha0. Focused core, desktop and real media/render
  checks pass. The clean sequential workspace run passed 580 tests with 9
  explicit fixture/manual ignores. Strict all-target/all-feature Clippy,
  formatting, Python syntax and diff checks pass.

## Limits

This deterministic V4 path composes owner-selected material; it does not choose
the best moments, generate footage, animate arbitrary scenes, transcribe audio,
publish or claim automatic aesthetic quality. The title renderer currently
requires LibreOffice and Poppler in addition to FFmpeg/FFprobe. Live provider
content quality, V2 clip selection, V5 motion/explainer generation,
Windows-native behavior, signing and public release remain separate gates.
