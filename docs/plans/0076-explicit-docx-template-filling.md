# Task0076: Explicit DOCX template filling

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances D5 locally without a live provider/model call, account
action, dependency installation or Windows claim.

## Problem

D5 promises that a repeatable document can be filled without losing track of
unknown values. Generic DOCX creation cannot prove that the selected template's
exact fields were handled, and silently removing a blank field creates a
misleadingly complete-looking result.

## Scope

1. Add `documents.template` for one frozen DOCX and an exact field set.
2. Support `{{field_key}}` placeholders in body, header and footer paragraphs,
   including placeholders split across multiple Word text runs.
3. Require every placeholder exactly once in the request and render null values
   as `[MISSING: field_key]`.
4. Preserve non-placeholder text and produce an editable DOCX plus a bounded
   audit JSON without private owner paths.
5. Add a localized native form that stages the selected template and explicit
   provided/missing values in Chat without sending or starting work.

## Non-goals

- No Word content controls, mail-merge fields, placeholders spanning paragraphs,
  rich value markup or semantic correctness claim.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- Deterministic coverage creates a real DOCX, fills provided fields, preserves a
  visible missing marker and reopens the result through the production inspector.
- A split-run fixture proves normal Word run fragmentation does not hide a valid
  placeholder.
- Omitted, extra, malformed or hidden missing values reject the job before
  publication.
- The native 800×560 form stages one DOCX and explicit values with zero Host
  commands.
- Retained native evidence is inspected; full workspace, strict Clippy,
  formatting, checksum and diff checks pass.
