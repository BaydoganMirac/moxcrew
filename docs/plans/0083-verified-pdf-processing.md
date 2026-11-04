# Task0083: Verified PDF processing

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances D2 without a provider/model call, account action,
dependency installation, publication or Windows claim.

## Problem

Moxcrew could render a PDF or extract a bounded text preview, but D2 still had
no typed PDF OCR flow and no merge operation. A generic request could silently
reorder inputs, hide empty scanned pages or return a file without proving that
the page count and source boundaries survived the merge.

## Scope

1. Add `pdf.extract {source,mode:text|ocr|auto,language?}`. Text mode supports
   up to 100 pages. OCR and automatic fallback are bounded to 12 pages and
   record the method, character count and text hash for every page.
2. Add `pdf.merge {sources:[{source}]}` for 2–12 PDFs and at most 300 pages.
   Preserve the supplied order, source hashes and exact page ranges.
3. Reopen the merged PDF, require the expected page count, compare each source
   block against the merged text, and use first/last boundary render equality
   when a source has no text layer. Reject encrypted, JavaScript-bearing or
   form-tree PDFs. Emit first/last PNG previews and a JSON manifest.
4. Expose Poppler structure/merge tools in local readiness, keep Tesseract
   optional for OCR, and route PDF resources away from image-only OCR actions.
5. Add localized native forms for extraction mode and explicit PDF merge order.
   The form only stages a Chat request; it never starts production directly.

## Non-goals

- No editing, signing, form filling, encryption removal or password bypass.
- No claim that OCR text is semantically correct without owner review.
- No dependency installation or provider/model execution during verification.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- Deterministic peers prove mixed text/OCR page attribution, exact source order,
  hashes, page limits and all merge/extraction outputs.
- A real two-source Poppler run produces a two-page A4 PDF, reopens with no
  JavaScript or encryption, extracts the expected page order and renders
  readable first/last pages.
- The 800×560 native form keeps its action visible and emits zero direct Host
  commands.
- Retained PDF/UI evidence is visually inspected; focused/full tests, strict
  Clippy, formatting, checksums and diff checks pass.

## Verification

- Deterministic core test:
  `pdf_extraction_and_merge_preserve_page_methods_order_and_source_evidence`.
- Native staging tests:
  `pdf_merge_request_keeps_owner_order_and_stages_no_direct_execution` and
  `resource_actions_are_format_specific_and_route_pdfs_to_bounded_extraction`.
- Real retained PDF: `docs/screenshots/pdf-processing/verified-board-packet.pdf`,
  SHA-256
  `c4bf52a2163afec794b23b9e80eb7d9e3aaab8bc40818d74d5b5b5cb01674090`.
  `pdfinfo` reports 2 A4 pages, no encryption, forms or JavaScript; independent
  `pdftotext` reopens page 1 as Executive summary and page 2 as Financial
  appendix.
- Visually inspected PDF renders:
  `merge-first.png`
  (`16d71eb54b3de14618ae2f6a73044732ec1cda4de7cfe46e8f86435819afeb58`)
  and `merge-last.png`
  (`39faecbd78708a940694d652db59c803030a969cc617a4785764fdc7450fca20`).
- Visually inspected native evidence:
  `pdf-merge-tr-dark.png` (1600×1120 RGBA), SHA-256
  `452d7e8d8843c6bbcdfb5758aff69dafb7df47846db354fd65bcf3860640b8ab`.
- Retained temporary runs: `/private/tmp/mox-pdf-d2.tgilFu` and
  `/private/tmp/mox-pdf-ui.d9959A`.
- `cargo test --workspace --locked -- --test-threads=1`: 567 passed, 6 ignored.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
  `cargo fmt --all -- --check` and `git diff --check`: passed.
- Toolchain: Rust/Cargo 1.99.1; Python 3.9.6.
- The real Poppler merge/render/text path is verified. Tesseract was not
  installed on this Mac, so OCR orchestration, page attribution and limits are
  verified with the deterministic executable peer rather than a live OCR
  quality claim.
