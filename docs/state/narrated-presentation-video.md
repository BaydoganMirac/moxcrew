# Narrated presentation video

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0085](../plans/0085-narrated-presentation-video.md)

**Project tools → Create outputs → Narrate presentation** now stages a bounded
typed request in Chat without starting production. The owner chooses one exact
PPTX, one to eight exact audio resources in visible filename order and a fixed
24, 25 or 30 fps output. The native form states that every slide needs speaker
notes and that a human must confirm the spoken words match those notes.

The local `presentations.narrate` worker accepts one approved audio clip per
slide in exact slide order. It rejects active-content decks, missing notes,
duplicate or mismatched slide numbers, non-audio streams, clips outside the
bounded duration and unsupported frame rates before publication. Every slide is
rendered through LibreOffice and Poppler, normalized to a 1280×720 MPEG-4/AAC
segment and concatenated into one MP4. The worker then probes both streams,
checks total duration, decodes the complete video and retains start, middle and
end frames. Its manifest pins source, note and audio hashes and intentionally
records `spokenTextVerified: false`.

## Verification

- Deterministic core coverage rejects wrong segment count/order and missing
  notes before encoding. Desktop coverage proves exact filename ordering, no
  direct Host execution and reachability at 800×560 in Turkish/English and both
  themes.
- A real three-slide noted deck plus three local system-voice WAV clips produced
  [presentation-video.mp4](../screenshots/presentation-narration/presentation-video.mp4),
  2,207,248 bytes, SHA-256
  `6178e65a941a66dce8f74cc2c5cc9f13ff686f58ddeb5e30547b8315ed8886d0`.
  FFprobe found MPEG-4 video at 1280×720 and AAC stereo audio at 48 kHz. Its
  15.579938-second duration matches 15.586757 checked narration seconds within
  the bounded tolerance, and FFmpeg decoded the full file successfully.
- The exact narration inputs are 4.878186, 4.049977 and 6.658594 seconds. Their
  respective SHA-256 hashes are
  `5c0723614ef7de567f28e8344dd9dc6a15a7a9e1cf8103e8e139e470a7fe20cc`,
  `d362ab7eb976fa4845b409caa4bc285b3572eb6cb2815c332bac76501cf9da13`
  and `0c47f2046f1b29367b4a89fc5f8d8aab668d7774a3ef00cc723d63250cef2f4c`.
- All three slide renders and the retained start, middle and end frames in
  [`docs/screenshots/presentation-narration/`](../screenshots/presentation-narration/)
  were inspected individually. They show the expected cover, statement and bar
  chart with no observed clipping or missing content. The manifest is SHA-256
  `50b89e97d366b893c550c69874b58b8e8970a8d13401fe152d03d2291930748c`.
- The real Turkish dark native capture
  [presentation-narration-studio-tr-dark.png](../screenshots/presentation-narration/presentation-narration-studio-tr-dark.png)
  is 2560×1760, 555,405 bytes and SHA-256
  `bf40b32b2eccf5d8ca32571121cf2877cfef6df736df4ec3aecde1c9fd9467af`.
  It shows the selected deck, three clips in exact numbered order, 30 fps and the
  visible staging action without clipping.
- The retained run used FFmpeg/FFprobe 9.0.2, Poppler `pdftoppm` 26.05.0 and
  LibreOfficeDev 26.8.0.0.alpha0. The full sequential workspace run passed 577
  tests with 8 explicit fixture/manual ignores. Strict all-target/all-feature
  Clippy, formatting, Python syntax and diff checks pass.

## Limits

Mox does not synthesize speech, clone a voice, transcribe clips or automatically
claim that spoken words match speaker notes. The retained audio used a local
macOS system voice only as real packaging evidence; content agreement still
needs human listening. The deck was rendered through LibreOffice, not opened in
Microsoft PowerPoint. Live provider/model quality, Windows-native behavior,
signing, public release and third-party playback compatibility remain separate
release gates.
