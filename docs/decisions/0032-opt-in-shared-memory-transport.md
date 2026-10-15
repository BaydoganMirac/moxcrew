# ADR 0032 Opt-in shared memory transport

Status: Accepted for Task0031

Date: 2026-10-15

Implement ADR0030's single publishing host and explicit readers in one personal
Cloud workspace, through separate memory protocol1. V4 local acceptance remains
canonical. V5 publication intents/attempts/acknowledgements are written by the
same local writer. Publishing freezes an owner-previewed accepted DTO; no source
text/origin records or ordinary operational serializers are extended.

Use immutable Cloud decision versions and a conditional-revision manifest of
current heads, with atomic predecessor removal on a published replacement.
Pagination of the current manifest supplies bounded recovery; a delta feed is
deferred. Server authentication, billing-derived entitlement and explicit role
are checked on every request. Local mapping binds connection identity; re-pairing
does not reuse another account's publication authority.

An exact authorized retry returns the original semantic receipt. Delivery has a
seven-day bound independent of five-minute local command admission. A recorded
uncertain attempt recovers its original outcome; it cannot be treated as never
sent. Reader caches are protected derived data with revision/freshness/current
labels; they cannot accept or replace local decisions. No automatic publication
occurs on acceptance, pairing or subscription.

Older Local releases cannot read V5 publication events. Close and back up the
complete data directory before first shared-memory action; rollback requires
that unchanged pre-upgrade copy. No development test migrates owner data.

See [protocol](../memory-protocol.md) and [Task0031](../plans/0031-cloud-memory-publication.md).
