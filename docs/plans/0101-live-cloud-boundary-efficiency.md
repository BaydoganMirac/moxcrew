# Task 0101: Live Cloud boundary efficiency

Status: Complete

Updated: 2026-11-06

Authorization: the owner asked for the live Local/Cloud distinction,
communication security and synchronization efficiency to be hardened. This
card authorizes repository code and tests only. It does not authorize a live
deployment, database migration, release, push or destructive cleanup.

## Problem

Protocol V2 correctly sends an allowlisted, rebuildable projection, but Local
currently uploads the complete bounded projection on every sync even when its
meaningful content is unchanged. General Cloud responses are bounded but Local
does not require the same `no-store` response guarantee already required by
the opt-in memory transport.

## Scope

1. Add an optional, declared V2 projection digest to the heartbeat. Older V2
   heartbeats remain valid and receive the existing acknowledgement shape.
2. When a new client supplies the digest, let Cloud return a bounded
   `projectionRequired` hint derived from the currently stored sanitized
   projection. Local skips only the redundant projection request; heartbeat,
   intent polling, receipt ordering and Local authority remain unchanged.
3. Require `cache-control: no-store` on Local/Cloud protocol responses and
   return it on all Cloud JSON errors as well as successes.
4. Keep exact schemas, size bounds, authorization, replay protection,
   revisions, credential handling and terminal receipts fail closed.

## Acceptance

- Legacy and digest-aware V2 heartbeat messages both validate.
- An unchanged projection uses heartbeat plus intent polling and does not send
  a projection request; a changed projection still sends the complete
  allowlisted projection.
- A missing `no-store` response is rejected as a protocol error without
  exposing the server body.
- Boundary tests cover forged digest input, legacy compatibility, changed and
  unchanged projections, denial, replay, revision and receipt behavior.
- Formatting, strict Clippy and the relevant Node/Rust suites pass.

## Limits

The digest covers only the already-sanitized projection and is not a Local
state hash. It never exposes source, paths, prompts, output, credentials or
project memory. This task does not add remote execution, background push,
WebSockets or Cloud authority over Local state.

## Completion evidence

- Protocol V2 accepts legacy and digest-aware heartbeats. The digest has a
  shared cross-language canonical vector, excludes `revision`, and covers only
  the allowlisted projection.
- A digest-aware mock proves the first sync uploads the projection and later
  unchanged syncs skip it while heartbeat, intent polling and receipt replay
  continue.
- Local rejects missing `no-store` before body use; Cloud emits `no-store` and
  `nosniff` for JSON errors without exposing internal failures.
- The seven Local Cloud-boundary tests, 71 Cloud tests and optimized Cloud build
  pass. Eleven real-PostgreSQL Cloud tests remain intentionally skipped without
  the external test database.
