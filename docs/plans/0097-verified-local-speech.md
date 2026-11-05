# Task0097: Verified local speech

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: the owner asked to continue closing missing product features after
the desktop interface overhaul, with Windows verification deferred to a Windows
PC. This card adds a bounded owner-visible A2 speech-generation path without a
remote model, account mutation, download or publication action.

## Problem

Moxcrew can transcribe, clean and assemble audio, but it cannot turn exact
owner-approved text into a checked local voiceover. A2 therefore has a catalog
entry without a typed production operation, native preparation flow or artifact
evidence.

## Scope

1. Add one typed `audio.speech` production operation using owner-configured
   Piper, an exact local ONNX voice model and its companion JSON configuration.
2. Accept an exact bounded title, script and language code. Require the model
   configuration language to match the requested language before synthesis.
3. Normalize the generated audio to a checked 48 kHz mono PCM WAV and emit
   canonical content, a script/model/config/output-hashed manifest and a concise review
   summary.
4. Add native Project production readiness and a Create form that stages this
   exact operation in Chat. The form never runs production directly and remains
   usable in the supported narrow desktop viewport in English and Turkish
   light/dark themes.
5. Route A2 through real Python, Piper, model, FFmpeg and FFprobe readiness
   instead of treating separately checked remote voice access as completion.

## Acceptance

- Title, script and language use strict bounds; unknown fields, malformed
  language codes, missing or oversized models/configurations and a configuration
  language mismatch fail closed.
- Piper receives the exact script through a private input file and is invoked as
  a direct child process, never through a shell or network request.
- The final WAV has exactly one audio stream, no video, 48 kHz mono PCM, bounded
  duration and a successful full decode. Exact script, model/config hashes,
  output hash and measured media evidence are retained without leaking absolute
  local paths.
- The output stays inside the existing production file/count/size bounds and
  records pronunciation, meaning, voice suitability and voice-model licence as
  explicit human review.
- Native actions stage the typed request in Chat and do not issue a Host
  production command. Geometry/action tests cover the smallest supported
  viewport and both locales/themes.
- Focused worker, routing, readiness, schema, UI and screenshot evidence pass;
  affected documentation and state are current; the local commit contains only
  intended files.

## Limits

This operation proves exact text transfer, local model identity, file integrity
and bounded audio structure. It does not prove pronunciation, semantic accuracy,
voice naturalness, accessibility, ownership or licence suitability. It does not
clone a voice, translate text, download a model, publish audio or complete A5.
Real voice quality and model licensing remain owner review. No provider/model
account call, upload, push, tag or release is authorized by this card. Native
Windows runtime verification remains deferred to the owner's Windows PC.
