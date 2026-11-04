# Task0071: Podcast editing package

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows feature gaps.
This card advances A4 locally without a provider/model call, account action or
new installed dependency.

## Problem

The A4 catalog promised playable edited segments, chapter information and a
package, while typed production exposed only whole-file cleanup and
transcription. Moxcrew could describe a podcast edit but could not execute one
bounded, inspectable chapter plan.

## Scope

1. Add typed audio.podcast for 1–12 ordered, frozen source segments.
2. Validate every source and time range with FFprobe before editing.
3. Produce a metadata-stripped playable M4A, ordered chapters.json and
   validation evidence.
4. Add a Turkish/English native form that freezes one Project recording and an
   explicit start | end | chapter title plan into Chat without starting work.
5. Reuse private scratch, source remapping, process deadlines and existing 25 MiB,
   16-output and 20-minute media limits.

## Non-goals

- No transcription, automatic silence removal, speaker detection, music sourcing
  or editorial quality claim.
- No TTS, voice cloning, publication, account action or new tool installer.
- No Windows-native result-access claim before Windows-PC verification.

## Acceptance

- Deterministic coverage proves ordering, recomputed chapter times, range denial
  and no scratch-path disclosure.
- The same proof passes the installed real FFmpeg and FFprobe.
- The 800×560 native form stages the exact segment plan but emits no Host command.
- The full workspace, strict Clippy, formatting and diff checks pass, with a
  retained native visual.
