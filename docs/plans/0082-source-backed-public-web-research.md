# Task0082: Source-backed public web research

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances R1 without a provider/model call, account action,
dependency installation, publication, form submission or Windows claim.

## Problem

Moxcrew can inspect one public page in an isolated browser, but R1 still lacks a
typed path that turns retrieved pages into an exportable research report. A
generic prompt can omit access dates, cite text that was never retrieved, hide
unknowns or lose the exact response identity behind a claim.

## Scope

1. Add `browser.research` for one to four public HTTP(S) sources. Reuse the
   existing public-address policy, fixed GET/HEAD interception, disabled page
   scripts, isolated profile and no-submit boundary.
2. Re-fetch every requested page during publication and retain its final URL,
   UTC access time, response SHA-256, extracted-text SHA-256 and truncation
   signal.
3. Require every fact or inference to cite a literal quote from the retrieved
   page. Inferences keep a review note; unknowns keep a reason and no invented
   evidence. Every selected source must be used.
4. Produce `research-report.md`, `sources.csv` and
   `research-evidence.json`. The CSV follows the bundled research skill's
   `claim,source_url,accessed_at,evidence_location` contract.
5. Add a localized native form that stages public URLs, title, research
   question and claims in Chat without retrieving, sending or starting work.

## Non-goals

- No private/authenticated page, existing browser profile, page script, form
  submission, download, publication, purchase or account action.
- No automatic truth, source-quality, consensus or inference-correctness score.
- No provider/model execution during verification.
- No Windows-native claim; Windows remains deferred to the requested PC test.

## Acceptance

- A fixed browser peer yields one retrieved source, UTC access metadata,
  response/text hashes, fact/inference/unknown claim classes and all three
  export files.
- A fabricated quote, private URL, unused source or invalid classification
  rejects before publication.
- The 800×560 native form stays visible and emits zero direct Host commands.
- Retained native evidence is inspected; focused/full tests, strict Clippy,
  formatting, checksum and diff checks pass.

## Verification

- The fixed browser policy test exercises real report/CSV/JSON generation with
  an injected deterministic CDP peer and rejects fabricated evidence. Existing
  policy cases continue to deny private/reserved addresses and non-read HTTP.
- The native test proves that the form only stages `browser.research`, empty
  evidence placeholders and the explicit inspect-first contract.
- `cargo test --workspace --locked -- --test-threads=1`: 564 passed, 6 ignored.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
  `cargo fmt --all -- --check` and `git diff --check`: passed.
- Toolchain: Rust/Cargo 1.99.1; Python 3.9.6.
- Retained native evidence:
  `docs/screenshots/web-research/web-research-tr-dark.png` (1600×1120 RGBA),
  SHA-256
  `27466dd9184e2c1c8aec87bbb459c497fdbe4accd95942521b3bbc0d41beffd3`.
- Retained temporary native run: `/private/tmp/mox-web-research-ui.Ogzqp5`.
- The typed policy, evidence and export path is verified with a deterministic
  browser peer. Real-site availability and source quality are intentionally not
  claimed without a separate live-network run.
