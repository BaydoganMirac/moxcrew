# Task0075: Attributed brief and action plan

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows product
gaps. This card advances D3 locally without a live provider/model call, account
action, dependency installation or Windows claim.

## Problem

D3 promises that notes become a useful brief, open questions and next actions.
The generic DOCX operation can write paragraphs, but it cannot prove where a
finding or action came from, or keep absent owners and dates explicit.

## Scope

1. Add `documents.brief` for 1–8 frozen supported text, PDF or image sources.
2. Require every finding and action to retain a literal quote from its exact
   source; independently compute the evidence line.
3. Keep unresolved questions explicit and preserve missing action owners or due
   dates as null rather than guessed placeholders.
4. Produce an editable DOCX plus evidence JSON with source hashes and no private
   owner paths.
5. Add a localized native form that stages frozen sources and title in Chat
   without sending, preparing or starting work.

## Non-goals

- No automatic truth, prioritization, staffing, deadline or completion claim.
- No dependency installation, Cloud/account action or Windows-native claim.

## Acceptance

- Deterministic coverage reopens the DOCX and proves two attributed findings,
  one unresolved question, two actions, explicit null owner/date, source hashes
  and evidence lines.
- Fabricated evidence and impossible calendar dates reject the entire job before
  publication.
- The native 800×560 form freezes only supported selected resources and issues
  zero Host commands.
- Retained native evidence is inspected; full workspace, strict Clippy,
  formatting, checksum and diff checks pass.
