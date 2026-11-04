# Task0080: Attributed multi-source synthesis

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances R3 locally without a live page request, provider/model
call, account action, dependency installation or Windows claim.

## Problem

R3 exists in the ability catalog, and D3 can make a source-attributed action
brief, but Moxcrew cannot yet prove that one finding actually combines multiple
documents or preserve conflicting source positions in an exportable synthesis.
A generic prompt may cite only one document, leave selected sources unused or
silently rewrite disagreement as consensus.

## Scope

1. Add `research.synthesis` for two to eight frozen text-extractable sources.
2. Require every selected source to be cited and at least one finding to bind
   literal evidence from two or more distinct sources.
3. Preserve disagreements as two or more source-distinct positions with literal
   quotes, hashes, UTF-8 byte offsets and line numbers.
4. Produce human-readable Markdown and machine-readable JSON without exposing
   private input paths. An empty disagreement list must explicitly say that it
   is not evidence of consensus.
5. Add a localized native form that stages sources, title and owner focus in
   Chat without sending, preparing or starting work.

## Non-goals

- No automatic truth, source-quality, causality or consensus judgment.
- No live URL retrieval, crawling or provider execution during verification.
- No action plan, staffing or due-date semantics already owned by D3.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- A three-source fixture yields an attributed cross-source finding and a
  two-sided disagreement while retaining exact source/text hashes and quotes.
- Fabricated quotes, unused or unknown sources, single-source-only findings and
  same-source disagreement positions reject before publication.
- The 800×560 native form stays visible, excludes unsupported resources and
  emits zero direct Host commands.
- Retained native evidence is inspected; focused/full tests, strict Clippy,
  formatting, checksum and diff checks pass.

## Verification

- The typed worker test covers three-source synthesis, two cross-source
  findings, a source-distinct disagreement, source/text hashes, literal quote
  locations, private-path redaction and the explicit no-consensus result.
- Fabricated evidence, unused sources, single-source-only findings and duplicate
  disagreement sources reject before files are published.
- The native 800×560 test proves the action is enabled and fully clickable,
  excludes an unsupported binary source, stages empty provider-fill sections
  and emits no Host command.
- `cargo test --workspace --locked -- --test-threads=1` passes with 561 tests;
  six explicit external-fixture/manual tests remain ignored. Loopback and owned
  process tests used the required local OS permissions.
- Strict workspace/all-target/all-feature Clippy, formatting and `git diff
  --check` pass.
- Retained Turkish dark native evidence:
  `docs/screenshots/research-synthesis/research-synthesis-tr-dark.png`
  (1600×1120 PNG, SHA-256
  `52113beff3c36d52e00b413041484bb3e9be46dca33ae8478b5cbcd216c20db4`).
