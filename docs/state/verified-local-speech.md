# Verified local speech

Updated: 2026-11-05

Status: Complete bounded local adapter and native preparation path; real voice quality and licensing review pending

Plan: [Task0097](../plans/0097-verified-local-speech.md)

**Project tools → Create outputs → Create local voiceover** now stages one exact
owner-approved title, script and locale in Chat without starting production. The
form names its local/no-download boundary, accepts locale codes such as `tr_TR`
or `en_US`, and keeps pronunciation, meaning, voice suitability and model licence
as explicit human review. The owner still sends the request, reviews the plan and
starts work separately.

The local `audio.speech` worker requires owner-configured Piper, an ONNX voice
model and its companion `<model>.onnx.json` configuration. Readiness checks the
executable, bounded model/config files and declared locale. Production rejects a
locale that does not exactly match the voice configuration, passes the exact
script through a private input file and invokes Piper directly without a shell,
network request or model download.

The synthesized source is probed and bounded before two-pass normalization. The
final `speech.wav` must reopen as one 48 kHz mono PCM stream, stay within 240
seconds and decode completely. `speech-content.json` preserves the exact script;
`speech-manifest.json` pins script, model, configuration and output hashes plus
stream/loudness evidence; `summary.md` keeps the performed and human checks
separate. Absolute tool/model paths never enter the deliverables.

## Verification

- Deterministic core coverage exercises the actual Piper CLI argument contract
  with a local protocol peer, including private exact-text input, companion
  configuration, streaming model/config hashes, FFprobe stream checks, two-pass
  normalization and full FFmpeg decode.
- The adapter rejects a missing companion configuration at setup, a voice/request
  locale mismatch and malformed locale input. It proves that the four outputs
  contain no private model path or filename and that the returned manifest hash
  matches the exact manifest bytes.
- Native desktop coverage proves exact title/script/locale staging, zero direct
  Host commands and scroll-reachable action controls at 800×560 in
  Turkish/English and light/dark themes.
- The A2 readiness profile now requires Python 3.9+, FFmpeg, FFprobe, Piper and a
  valid Piper ONNX/config pair. The shared Project tool picker distinguishes
  executable selection from model-file selection and installs nothing.
- The all-target/all-feature Rust workspace gate passed 607 tests with 16
  explicitly ignored external/manual fixtures. Strict Clippy passed with
  warnings denied, and the Node 24.22.1 legacy parity suite passed 153/153.
- Python syntax, Rust formatting and repository diff checks passed. No real
  voice model was downloaded or invoked.
- The retained Turkish dark-theme native screenshot is
  `docs/screenshots/local-speech/local-speech-form-tr-dark.png`: 1600×1120,
  301,088 bytes, SHA-256
  `345c2c11da8c3aafd610782ffe6a55a9eb6a7fbcf7b5ec2a71d49ffa7f1b5aaf`.
  Automated geometry coverage separately proves the scrolled action remains
  reachable at the supported 800×560 minimum in both locales and themes.

## Limits

This closes a bounded local A2 adapter and owner-visible preparation path, not a
voice-quality or licensing gate. The protocol peer proves command, validation,
normalization, file and evidence behavior; it does not prove pronunciation,
naturalness, emotional tone, semantic accuracy, accessibility or permission to
redistribute a selected voice. The owner must review the individual model's
licence and listen to real output. Voice cloning, model discovery/download,
translation, A5 translated narration, publication, Windows-native behavior,
signing and public release remain separate gates.
