# Branded visual presentations

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0084](../plans/0084-branded-visual-presentations.md)

**Project tools → Create outputs → Design presentation** now stages a bounded,
theme-aware presentation request in Chat. The owner chooses the native Office
theme, an optional Project logo, exact typed slide rows and whether every slide
must be rendered before delivery. The form does not start a model or production
operation directly. A separate **Apply presentation brand** request targets one
exact Project PPTX and preserves its slides, notes, images and relationship parts.

The local presentation worker supports cover, statement, content, split and
bar-chart layouts. Text remains editable DrawingML text, charts remain editable
native shapes and values, and PNG/JPEG artwork remains an embedded frozen input.
The theme contract covers primary, accent, background, text, muted and font
values. Invalid contrast, excessive density, unsupported layouts, unsafe active
content and unbounded render requests fail before publication.

`presentations.revise` replaces only the selected slide while checking that every
non-target slide XML part remains byte-identical. `presentations.brand` changes
native Office theme parts while checking that every slide, note, image and
relationship part remains byte-identical. Its manifest distinguishes
scheme-aware elements from hard-coded colors, so arbitrary third-party decks are
not described as fully adapted when explicit colors remain.

## Verification

- Focused core coverage proves editable typed layouts, native bar values, notes,
  exact non-target revision preservation, exact brand-part preservation, theme
  replacement, density rejection and contrast rejection.
- The real retained four-slide deck
  [presentation.pptx](../screenshots/presentations/presentation.pptx) is 1,516,228
  bytes, SHA-256
  `8e3a382a6d2e157d15da91c89f5ab3abce66af18b2dcd51b8278b5d588e83d55`.
  It contains one embedded local Mox image, native theme-aware text and bars,
  and speaker-note parts on slides 1 and 4.
- The product worker opened the deck with the configured LibreOffice runtime and
  rendered a four-page 16:9 PDF plus four 1280×720 PNGs through Poppler. Every
  retained slide image was inspected individually; no clipping, missing artwork,
  broken relationship or unreadable chart was observed. The exact render evidence
  and checked layout/value manifest live in
  [`docs/screenshots/presentations/`](../screenshots/presentations/).
- The native form stages zero direct Host commands. Focused desktop coverage
  proves exact resource selection, chart parsing, create/brand request contents
  and scroll reachability at 800×560 in Turkish/English and light/dark themes.
- The full workspace passes sequentially with 574 tests and 7 explicit
  fixture/manual ignores. Strict workspace/all-target/all-feature Clippy,
  formatting, Python syntax and diff checks pass.
- A real 2560×1760 Turkish dark native capture is retained as
  [presentation-studio-tr-dark.png](../screenshots/presentations/presentation-studio-tr-dark.png),
  SHA-256
  `d23dde7b1ca75028dcf0190ee6ac7fe83eb237105e9945184a138073fb5acaa5`.

## Limits

P5 is implemented separately in
[narrated presentation video](narrated-presentation-video.md). The worker does
not automatically rewrite arbitrary third-party slide layouts, smart art,
animations or hard-coded colors. The retained deck was opened and rendered with
LibreOffice, not checked inside Microsoft PowerPoint. Live provider/model
quality, Windows-native behavior, signing and public release evidence remain
separate gates.
