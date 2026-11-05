# Task0084: Branded visual presentations

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps and make Mox excellent. This card advances the presentation matrix without
a provider/model call, account action, dependency installation, publication or
Windows claim.

## Problem

Moxcrew can create an editable PPTX and replace one selected slide, but the
typed worker only produces a title and bullet list. It has no reusable visual
layout system, no theme-aware brand contract, no chart slide and no way to
request slide renders in the same checked production operation. The existing
revision proof also checks extracted text rather than exact preservation of the
untouched slide parts.

## Scope

1. Extend `presentations.create` with bounded cover, statement, content, split
   and bar-chart layouts. Keep all text and chart data editable native Office
   objects, with readable type sizes and bounded content density.
2. Add an explicit theme contract for primary, accent, background, text, muted
   and font values. Optional PNG/JPEG logo and slide image inputs remain frozen
   Project resources. Emit a presentation manifest containing the applied theme,
   slide layouts, source hashes, chart values and verification results.
3. Strengthen `presentations.revise` so the selected slide can use the same
   visual grammar while every non-target slide XML part remains byte-identical.
4. Add `presentations.brand {source,brand,render?}`. It updates the native Office
   theme and preserves every slide, note, image and relationship part exactly.
   The manifest reports how many scheme-aware and explicit-color elements were
   found so a partial third-party-template adaptation cannot be overstated.
5. When `render` is true, use the already configured private LibreOffice and
   Poppler path to render every slide of a bounded 1–12 slide deck to PDF and PNG.
   Missing tools fail explicitly; a generated file alone is not called visually
   checked.
6. Add a localized native presentation form that stages the exact operation in
   Chat. It must expose layout, slide content, brand, render choice and brand
   application without directly starting production.

## Non-goals

- No narrated presentation or speech generation; P5 remains separate.
- No claim that an arbitrary third-party template becomes fully reauthored when
  it contains hard-coded colors or unsupported objects.
- No dependency installation, provider/model execution, publication or Cloud
  protocol change.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Deterministic tests prove strict argument bounds, editable layout/chart text,
  exact non-target slide preservation, exact slide preservation during rebrand,
  theme replacement, notes retention and honest explicit-color coverage.
- A real four-slide branded deck opens in LibreOffice, renders all slides, keeps
  its chart values and speaker notes, and produces visually inspected full-size
  PNGs. A selected-slide revision changes only that slide part.
- The 800x560 native form remains reachable in Turkish and English, both themes,
  and emits no direct Host command.
- Focused/full tests, strict Clippy, formatting, checksums and diff checks pass.
