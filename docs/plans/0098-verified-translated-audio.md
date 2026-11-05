# Task0098: Verified translated audio

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: the owner approved the complete forty-ability release target and
asked to continue closing the remaining product gaps, with Windows verification
deferred to a Windows PC. This card adds the bounded owner-visible A5 path without
an automatic translation claim, remote model call, download or publication.

## Problem

Moxcrew can preserve a source-pinned written translation and can synthesize exact
approved local speech, but it cannot produce a checked translated audio version
whose duration is bound to the original recording. A5 therefore has no typed
production operation, native preparation flow or retained timing evidence.

## Scope

1. Add one typed `audio.dub` operation for an exact Project audio source, exact
   source transcript, exact owner-approved translation and target locale.
2. Require the owner to confirm that the source/translation meaning pair was
   reviewed before the request can be staged or executed. Do not translate text.
3. Synthesize only the approved translation with the configured local Piper
   voice, then fit it to the checked source duration within an explicit bounded
   pace range and normalize it to a 48 kHz mono PCM WAV.
4. Emit canonical paired content, source/script/model/config/output hashes, pace
   and duration evidence, plus a review summary with automatic and human checks
   kept separate.
5. Add an A5-ready native Project form that stages the exact operation in Chat,
   never starts Host production directly and remains reachable at 800×560 in
   English/Turkish light/dark themes.

## Acceptance

- Unknown fields, malformed locales, missing/unsupported sources, unreviewed
  meaning, empty/oversized text, locale mismatch and unsafe pace ratios fail
  closed before a deliverable is accepted.
- Piper receives the exact translated text through a private input file and is
  invoked directly without a shell, network request, voice cloning or download.
- The source and raw voiceover are probed. The final WAV has one `pcm_s16le`,
  48 kHz mono stream, matches the source duration within the fixed tolerance,
  decodes completely and stays within one LU of the target loudness.
- Deliverables retain the exact source/translated text pair and hashes without
  leaking absolute paths or local model filenames. Manifest wording records the
  owner meaning review without turning it into an automatic semantic claim.
- Native tests prove exact staging, no direct execution, explicit meaning review
  and action reachability at the minimum viewport in both locales/themes.
- Focused tests, full repository gates, state documentation and one retained
  native screenshot pass; the local commit contains only intended files.

## Limits

This operation time-fits one approved translation to one source recording. It
does not translate, transcribe, perform lip sync, align individual sentences,
clone a voice, download a model, publish audio or prove pronunciation,
naturalness, accessibility or licence suitability. Extreme duration differences
are rejected instead of forcing an unusable pace. Real voice quality and the
selected model licence remain owner review. No live provider/account call,
upload, push, tag or release is authorized. Windows-native verification remains
deferred to the owner's Windows PC.
