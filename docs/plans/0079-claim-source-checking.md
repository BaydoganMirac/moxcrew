# Task0079: Claim/source checking

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances R4 locally without a live page request, provider/model
call, account action, dependency installation or Windows claim.

## Problem

R4 exists in the ability catalog and individual literal evidence can be saved,
but Moxcrew cannot yet produce one checked report that keeps supported,
disputed and unresolved claims visibly tied to the exact frozen sources that
were inspected. A generic prompt could copy a claimed status, invent evidence
or hide which sources were actually checked.

## Scope

1. Add `research.claims` for one to eight frozen text-extractable sources and a
   bounded set of claims.
2. Verify every evidence quote literally against its selected source and record
   exact source/text hashes, UTF-8 byte offsets and line numbers.
3. Derive each status locally: any dispute is disputed, otherwise any support
   is supported, otherwise it is unresolved with an explicit reason.
4. Produce human-readable Markdown and machine-readable JSON without exposing
   private input paths.
5. Add a localized native form that stages sources, title and claims in Chat
   without sending, preparing or starting work.

## Non-goals

- No automatic truth score, semantic entailment or source-quality judgment.
- No live URL retrieval, crawling or provider execution during verification.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- Supported, disputed and unresolved examples produce a source-hashed report;
  conflicting support/dispute is retained and classified disputed.
- Fabricated quotes, unchecked evidence sources, duplicate claims and missing
  unresolved reasons reject before publication.
- The 800×560 native form stays visible, excludes unsupported resources and
  emits zero direct Host commands.
- Retained native evidence is inspected; focused/full tests, strict Clippy,
  formatting, checksum and diff checks pass.

## Verification

- The typed worker test covers supported, conflicting disputed and unresolved
  claims, Unicode UTF-8 offsets, hashes, report files, private-path redaction,
  fabricated quotes, unchecked sources, missing reasons and duplicate claims.
- The native 800×560 test proves the action is enabled and fully clickable,
  excludes an unsupported binary source, stages the exact empty evidence
  contract and emits no Host command.
- `cargo test --workspace --locked -- --test-threads=1` passes with 559 tests;
  six explicit external-fixture/manual tests remain ignored. Loopback and owned
  process tests used the required local OS permissions.
- Strict workspace/all-target/all-feature Clippy, formatting and `git diff
  --check` pass.
- Retained Turkish dark native evidence:
  `docs/screenshots/claim-check/claim-check-tr-dark.png` (1600×1120 PNG,
  SHA-256 `d26c8e64ca0f921c5300022ed9a168f1cb0380cc6d8cf4d30cb22e474035c001`).
