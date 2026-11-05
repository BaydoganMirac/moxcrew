# ADR 0053: Moxcrew pairing-code prefix

Status: Accepted by owner direction

Date: 2026-11-05

## Decision

New Moxcrew Cloud pairing codes use the visible `MX-` prefix. Cloud accepts the
legacy `PF-` prefix only during the compatibility window so an already displayed
one-time code is not broken. Pairing codes remain random, ten-minute,
single-use values whose plaintext is never stored by Cloud.

## Consequences

This changes no endpoint, message field, credential, or local authority rule.
Local already treats the pairing code as bounded opaque text, so existing
desktop builds can submit `MX-` codes without a runtime change.
