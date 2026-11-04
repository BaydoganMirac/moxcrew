# Task0081: Evidence-backed owner-criteria comparison

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances R2 locally without a live page request, provider/model
call, account action, dependency installation or Windows claim.

## Problem

R2 exists in the ability catalog, but Moxcrew cannot yet prove that the same
owner criteria were applied to every option. A generic comparison prompt may
omit weak cells, mix recommendation with uncertainty, silently score prose or
publish unsupported claims.

## Scope

1. Add `research.compare` for one to eight frozen text-extractable sources,
   two to twelve options and one to twelve owner criteria.
2. Require every option to have the exact same criterion keys. Confirmed and
   uncertain cells require literal source evidence; missing cells remain null.
3. Verify quotes and retain source/text hashes, line numbers and UTF-8 byte
   offsets without exposing private source paths.
4. Keep the recommendation option, reason, owner-criterion basis and explicit
   uncertainty in separate fields and report sections. A null recommendation
   remains valid when evidence is insufficient.
5. Add a localized native form that stages sources, title, decision question,
   criteria and options in Chat without sending, preparing or starting work.

## Non-goals

- No automatic ranking, weighting, truth, source-quality or procurement score.
- No live URL retrieval, crawling or provider execution during verification.
- No claim that a recommendation is objectively correct or complete.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- A two-option, three-criterion fixture yields an even comparison table with
  literal evidence, source hashes and distinct Recommendation/Uncertainty
  sections.
- Fabricated quotes, uneven criterion sets and recommendations based on missing
  cells reject before publication.
- The 800×560 native form stays visible, excludes unsupported resources and
  emits zero direct Host commands.
- Retained native evidence is inspected; focused/full tests, strict Clippy,
  formatting, checksum and diff checks pass.

## Verification

- The typed worker test covers two options, three owner criteria, confirmed and
  uncertain cells, exact quote locations, hashes, redaction and report shape.
- Negative cases cover invented evidence, uneven tables and an unsupported
  recommendation basis.
- The native test proves the staged request contains every option × criterion
  cell, explicit missing placeholders and separate recommendation uncertainty,
  while issuing no Host command.
- `cargo test --workspace --locked -- --test-threads=1` passes with 563 tests;
  six explicit external-fixture/manual tests remain ignored. Loopback and owned
  process tests used the required local OS permissions.
- Strict workspace/all-target/all-feature Clippy, formatting and `git diff
  --check` pass with Rust/Cargo 1.99.1. Frozen UTF-8 text fixtures run through
  Python 3.9.6 and require no optional extraction engine.
- Retained Turkish dark native evidence:
  `docs/screenshots/research-comparison/research-comparison-tr-dark.png`
  (1600×1120 PNG, SHA-256
  `60d994ed0ccdecb5e985c980660c4926dc5f9c74d846b7ad13a347a45e98cec7`).
- Disposable native state is retained at
  `/private/tmp/mox-research-comparison-ui.RKIXvU`; it was not deleted without
  owner approval.
