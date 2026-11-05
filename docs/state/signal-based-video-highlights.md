# Signal-based video highlights

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0087](../plans/0087-signal-based-video-highlights.md)

**Project tools → Create outputs → Find video highlights** now stages one exact
video plus a bounded target duration, fixed clip length and balanced,
visual-motion or audio-energy emphasis. It does not start production directly.

The local `video.highlights` worker probes the complete source, samples 32×18
grayscale frames at 1 fps and mono audio at 200 samples per second, then scores
bounded candidate ranges from frame change and RMS energy. It greedily selects
non-overlapping ranges, keeps their exact component/combined scores and renders
them chronologically to a new 1280×720, 30 fps MPEG-4 output. The output is
probed and completely decoded; its edit plan, source/output hashes and one frame
per selected range are retained. The manifest explicitly requires human meaning
review because motion and loudness are ranking signals, not quality judgments.

## Verification

- Deterministic core coverage rejects an unknown emphasis before any tool call.
  Desktop coverage proves exact source freezing, no direct Host command and
  action reachability at 800×560 in Turkish/English and both themes.
- Real retained output
  [highlights.mp4](../screenshots/video-highlights/highlights.mp4) is 1,194,957
  bytes and SHA-256
  `178cb9115e019205f7abd7c16a275820b78d5c8036fcfa875df5ead8d23d0bda`.
  FFprobe found 1280×720 MPEG-4 video at 30 fps and AAC stereo audio at 48 kHz.
  Its 8.021355-second duration matches the requested 8 seconds and FFmpeg
  decoded the complete file.
- The 15.579938-second exact source hash is
  `6178e65a941a66dce8f74cc2c5cc9f13ff686f58ddeb5e30547b8315ed8886d0`.
  Balanced 55% motion/45% audio ranking evaluated seven candidates and selected
  seconds 4–8 and 8–12. The retained frames were inspected and show distinct,
  intact source content without clipping. The selection manifest SHA-256 is
  `c3c4c79d38cec5690572d7e460ef06ae8970f21f32f08d3126bd16abc725e87e`.
- The real 2560×1760 Turkish dark native capture
  [video-highlights-studio-tr-dark.png](../screenshots/video-highlights/video-highlights-studio-tr-dark.png)
  is SHA-256
  `8b1f70c3013569871390c46d6721b4ae12319c6cb1df54c1c34ad579185b0178`.
  It shows the exact source, target/clip durations, all three signal policies and
  the staging action without clipping or horizontal overflow.
- Focused deterministic, desktop and real-media checks pass. The clean
  all-target/all-feature workspace run passed 583 tests with 10 explicit
  fixture/manual/real-tool ignores. Strict Clippy, formatting, Python syntax and
  diff checks pass.

## Limits

This closes a deterministic V2 signal-selection path, not semantic highlight
understanding. It does not transcribe speech, identify people, infer emotion or
importance, generate footage, publish, or claim automatic aesthetic quality.
V5 generative motion, live provider content quality, Windows-native behavior,
signing and public release remain separate gates.
