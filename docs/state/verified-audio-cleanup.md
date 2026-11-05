# Verified audio cleanup

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0089](../plans/0089-verified-audio-cleanup.md)

**Project tools → Create outputs → Clean and level audio** now stages one exact
Project recording in Chat without starting production. The owner chooses
level-only, gentle or voice treatment and one visible target from -24, -20, -16
or -14 LUFS. The native form accepts only an available audio/video resource and
preserves the source, treatment and target in the typed request.

The local `audio.cleanup` worker accepts mono/stereo input up to two minutes,
measures the unmodified source, applies only the selected fixed filter chain and
two-pass loudness normalization, and produces a new 48 kHz PCM WAV without
mutating the source. It re-probes and fully decodes the output, requires duration
and channel preservation and rejects a result more than 1 LU from target. Its
manifest pins both hashes, streams, policy and before/after measurements.

## Verification

- Deterministic core coverage rejects unknown treatment and out-of-range LUFS
  before an external tool runs. Desktop coverage proves exact source/policy
  freezing, no direct Host command and action reachability at 800×560 in
  Turkish/English and both themes.
- The retained real-tool fixture
  [source-fixture.wav](../screenshots/audio-cleanup/source-fixture.wav) is a
  12-second generated stereo tone/noise recording at 48 kHz, 2,304,078 bytes and
  SHA-256
  `c64f702097fd529e4f46048b4792dc3eefb3acada08244d5a5134fc173fe01ea`.
  It measured -36.74 LUFS with -31.08 dB true peak before processing.
- Real retained output
  [cleaned-audio.wav](../screenshots/audio-cleanup/cleaned-audio.wav) is 2,304,078
  bytes and SHA-256
  `861f6b5e53e5711bcec4d072179fe58cc879a7052f0e03c19830e333b3cebbf7`.
  FFprobe found PCM 16-bit stereo at 48 kHz and exactly 12.000000 seconds.
  FFmpeg decoded the complete file; the output measured exactly -16.0 LUFS,
  -12.56 dB true peak and 0.1 LU loudness range. Duration and channel count match
  the source and the target difference is 0.0 LU.
- The source/output/measurement manifest SHA-256 is
  `4c7b77a111f0ab67378a8e7fe746ea67500bc2075a2d481b3a661fe47de18ba7`.
  It keeps subjective-quality review false: the deterministic tone/noise fixture
  proves the signal path and measurement contract, not human speech quality.
- The real 2560×1760 Turkish dark native capture
  [audio-cleanup-studio-tr-dark.png](../screenshots/audio-cleanup/audio-cleanup-studio-tr-dark.png)
  is 479,906 bytes and SHA-256
  `ab1f80fb9617b41bb442e1929d3e607b2ef30f59ec7e3b9faf4c6a6be11d9184`.
  It shows the exact source, all three treatments, four targets and staging action
  without clipping or horizontal overflow.
- The retained run used FFmpeg/FFprobe 9.0.2 and Python 3.9.6. Focused core,
  desktop and real-media checks pass. The sequential all-target/all-feature
  workspace run passed 589 tests with 12 explicit real-tool, PostgreSQL or manual
  fixture ignores and no failures. Strict all-target/all-feature Clippy with
  warnings denied, formatting, Python syntax and diff checks pass.

## Limits

The presets provide bounded high-pass/noise reduction and measured leveling;
they do not isolate speakers, remove reverb, repair clipped speech or prove that
the result sounds better. The retained source is synthetic and no subjective
listening acceptance is claimed. Inputs longer than two minutes, more than two
channels, speech generation, transcription, translation, publication,
Windows-native behavior, signing and public release remain separate gates.
