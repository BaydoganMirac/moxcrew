# ADR0046: Explicit local delivery archives

Status: Accepted within owner-authorized Task0047

Date: 2026-10-19

Composed Project work produces multiple files. Add one native owner export action
that packages explicitly selected, hash-pinned outputs from the same Project.
The Host alone resolves source directories and validates identities; UI paths
and output text cannot grant access. Read all bounded source bytes and verify
hashes before creating any destination.

Use a bounded standard uncompressed UTF-8 ZIP and a readable provenance manifest.
Reuse the native checked export destination and existing V12 requested/receipted
owner command mechanism. No new persisted domain entity, provider dependency,
runtime process, external account action or Cloud projection is needed. Refuse
existing/internal targets and retain the existing fail-closed platform boundary.
The original implementation allowed an I/O failure to leave a partial file.
[ADR0051](0051-atomic-owner-export-publication.md) supersedes that Unix export
detail: stage privately, publish atomically without replacement and never unlink
the requested public path.

This makes existing outputs easier to use; it does not establish new generation
abilities or complete the entire1.0.0 release matrix.
