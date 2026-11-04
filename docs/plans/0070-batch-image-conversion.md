# Task0070: Batch image conversion

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to complete the remaining non-Windows product
gaps. This card advances the approved I3 ability without a provider/model call
or a new installed dependency.

## Problem

The I3 recipe promised batch image conversion with per-file failures, but the
typed production worker exposed only one-image resize. A model could describe a
batch while Moxcrew lacked one bounded operation that actually converted every
selected source and kept partial failures inspectable.

## Scope

1. Add a typed `images.batch` operation for 2–12 frozen Project images.
2. Convert each supported PNG/JPEG/WebP source to a bounded PNG while preserving
   its aspect ratio.
3. Return an ordered result for every source; a malformed/unsupported file must
   not erase successful siblings.
4. Add a native Turkish/English Create outputs entry that freezes exact selected
   Project resources into Chat without starting work.
5. Reuse configured FFmpeg, private scratch, source remapping, process deadline,
   output limits and metadata-safe PNG validation.

## Non-goals

- No image generation, semantic editing, background removal or quality score.
- No arbitrary codec/format/quality options or new dependency installer.
- No provider/model/account call, task auto-start, Cloud payload or Windows
  runtime claim.

## Acceptance

- Bounds, source order, partial failure, output validation and metadata removal
  are covered by a deterministic test and the installed real FFmpeg.
- The desktop stages two selected immutable resources and exact dimensions but
  emits no Host mutation or task start.
- The full workspace/all-feature suite, strict Clippy, formatting and diff checks
  pass, and the native form receives retained visual evidence.
