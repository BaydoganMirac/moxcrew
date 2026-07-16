# Host-to-Cloud protocol

Status: implemented V1 contract

Updated: 2026-07-16

This public document defines the trust contract between a Patchfleet local host
and Patchfleet Cloud. The public repository owns the contract. The private
Cloud repository implements it.

## Invariants

- The host initiates every network connection.
- Local events and provider state are canonical.
- Cloud projections are sanitized and rebuildable.
- Cloud sends intent records, never executable shell text.
- The local host may reject any intent.
- Every accepted intent ends with one durable terminal receipt.
- Unknown schema versions fail closed.

## Data classes

### Allowed by default

- opaque host, workspace, work-item, run, intent, and receipt identifiers;
- user-chosen host display name;
- Patchfleet version and operating-system family;
- provider identifier, availability, version, and capability flags;
- lifecycle status and coarse timestamps;
- queue position, retry count, and terminal reason codes;
- owner-authored text that was originally entered in Patchfleet Cloud;
- sanitized command receipts.

### Allowed only with explicit opt-in

- a user-defined project alias;
- local-origin work-item title or description;
- fine-grained performance diagnostics;
- support bundles that have a preview and expiry.

### Forbidden in default Cloud payloads

- source code, diffs, patches, generated files, or repository archives;
- raw prompts, model responses, transcripts, reasoning, or tool output;
- terminal stdout or stderr;
- absolute or home-relative filesystem paths;
- environment variables, configuration files, tokens, cookies, credentials,
  model keys, or pairing secrets;
- Git remote URLs containing credentials;
- arbitrary provider-native payloads.

Sanitization is an allowlist projection. It is not a collection of regular
expressions applied after serializing the complete local object.

## Pairing

1. An authenticated Cloud user creates a short-lived, single-use pairing code.
2. The local user enters or opens that code in Patchfleet.
3. The host sends the code, host public identity, app version, and protocol
   version over HTTPS.
4. Cloud consumes the code and returns an opaque host identifier plus a
   revocable host credential.
5. The V1 host stores the credential in its private local data directory with
   owner-only file permissions. Platform key stores may replace this later.

The host persists its installation identifier before the consume request. If
the response is lost, a later code reuses that identifier so Cloud revokes the
orphaned credential while re-pairing.

Pairing does not transfer provider credentials. Re-pairing or revocation
invalidates the old host credential.

## Version 1 resources

The first implementation exposes these HTTP resources:

- POST /api/v1/pairings/consume
- POST /api/v1/hosts/{hostId}/heartbeat
- PUT /api/v1/hosts/{hostId}/projection
- GET /api/v1/hosts/{hostId}/intents?after={cursor}
- POST /api/v1/hosts/{hostId}/receipts

Route names, message fields, and the trust boundary are part of version 1 and
require a protocol decision to change.

Polling is the V1 delivery mechanism because it works through ordinary outbound
HTTPS and is easy to recover. Server-sent events may reduce latency later;
polling remains the recovery path.

## Exact messages

All records reject unknown fields. Identifiers are opaque ASCII strings of 1
to 256 characters. Timestamps are canonical UTC ISO 8601 strings. Request
bodies are JSON, carry `Content-Type: application/json`, and are limited to
64 KiB.

Pairing consumes:

```json
{
  "schemaVersion": 1,
  "messageId": "opaque",
  "occurredAt": "2026-07-16T00:00:00.000Z",
  "pairingCode": "PF-0123456789AB",
  "installationId": "opaque",
  "displayName": "user chosen, 1 to 80 characters",
  "appVersion": "0.1.0",
  "protocolVersion": 1,
  "osFamily": "darwin|linux|windows"
}
```

It returns `schemaVersion`, `hostId`, `workspaceId`, `credential`, and
`protocolVersion`. The pairing code and credential appear only in this TLS
exchange; Cloud persists their SHA-256 digests, never their plaintext.

Heartbeat uses the common `schemaVersion`, `messageId`, `hostId`, and
`occurredAt` fields plus this exact payload:

```json
{
  "appVersion": "0.1.0",
  "protocolVersion": 1,
  "lastLocalSequence": 0,
  "health": "ok"
}
```

`health` is `ok` or `degraded`.

Projection uses the same envelope and this exact payload:

```json
{
  "revision": 0,
  "providers": [{
    "providerId": "codex",
    "state": "available",
    "version": "semantic-version-or-null",
    "capabilities": { "recentObservation": true, "explicitLiveStatus": true },
    "observedAt": "2026-07-16T00:00:00.000Z"
  }],
  "workItems": [{
    "workItemId": "opaque",
    "providerId": "codex",
    "status": "queued",
    "revision": 1,
    "queuePosition": 0,
    "createdAt": "2026-07-16T00:00:00.000Z"
  }],
  "runs": [{
    "runId": "opaque",
    "workItemId": "opaque",
    "providerId": "codex",
    "status": "running",
    "revision": 1,
    "startedAt": "2026-07-16T00:00:00.000Z",
    "terminalAt": null
  }],
  "workItemsTruncated": false,
  "runsTruncated": false
}
```

Projection contains no receipts: receipts use their dedicated durable route.
It contains at most three providers, 32 non-terminal work items, and 32
non-terminal runs. Terminal command outcomes use the receipt route instead of
growing the current projection without bound. The truncation flags are `true`
when the host has more current records than the corresponding list carries, so
Cloud never presents a bounded view as complete.
Cloud replaces a host projection only when `revision` is strictly newer. An
exact retry of the current revision is acknowledged; a conflicting equal or
older revision is rejected.

Intent polling returns:

```json
{
  "schemaVersion": 1,
  "hostId": "opaque",
  "cursor": 4,
  "intents": [{
    "sequence": 4,
    "intent": {
      "schemaVersion": 1,
      "intentId": "opaque",
      "idempotencyKey": "opaque",
      "type": "cancel_run",
      "actorId": "cloud-owner",
      "createdAt": "2026-07-16T00:00:00.000Z",
      "expiresAt": "2026-07-16T00:05:00.000Z",
      "payload": { "runId": "opaque", "expectedRunRevision": 1 }
    }
  }]
}
```

Receipt delivery uses the common envelope and an exact payload containing one
to 20 existing local command receipts:

```json
{
  "receipts": [{
    "schemaVersion": 1,
    "intentId": "opaque",
    "idempotencyKey": "opaque",
    "commandType": "cancel_run",
    "outcome": "applied",
    "reasonCode": "RUN_CANCELLED",
    "completedAt": "2026-07-16T00:00:01.000Z",
    "workProjectionRevision": 3,
    "workItemId": "opaque-or-null"
  }]
}
```

Successful message writes return `{ "schemaVersion": 1, "accepted": true }`.
Cloud records bounded message identifiers so an exact replay is acknowledged
without applying the write twice; a conflicting replay is rejected.

Cloud rejects unknown hosts, replayed pairing codes, invalid credentials,
unsupported versions, and messages that violate size or schema limits.

## Heartbeat and projection

Heartbeat proves liveness and reports only app/protocol versions, last local
sequence, and coarse health.

Projection carries the latest allowed operational view. It is versioned and may
be replaced by a later projection from the same host. Cloud must not turn a
projection back into canonical local events.

The host owns monotonically increasing projection revisions. Cloud rejects an
older revision rather than regressing visible state.

## Command intent

An intent contains:

- opaque intentId and idempotencyKey;
- hostId and workspace authorization context;
- allowlisted command type;
- versioned typed payload;
- createdAt and expiresAt;
- expected local revision when the action depends on current state;
- actor identity suitable for an audit trail.

Version 1 supports exactly one Cloud-authored command type: `cancel_run`.
Local-only command types are not part of the Cloud protocol.

Cloud creates `cancel_run` intents with a lifetime of at most five minutes. A
host rejects a longer lifetime or a `createdAt` more than 60 seconds ahead of
its current clock. An already-expired valid intent still receives a durable
`expired` receipt.

There is no execute_shell, run_script, upload_repository, or generic provider
command. New intent types require a protocol decision and redaction review.

## Intent application

The local host validates, in order:

1. credential and host identity;
2. supported schema and command type;
3. expiry;
4. idempotency;
5. target existence and expected revision;
6. provider capability;
7. local safety policy.

Only then may it attempt a side effect. The host records requested, accepted or
rejected, and terminal outcome as local events.

## Receipt

A terminal receipt has one of four outcomes:

- applied;
- rejected;
- expired;
- failed.

It includes the intent and idempotency identifiers, timestamps, a stable reason
code, resulting sanitized revision when applicable, and no raw provider output.
Retrying delivery of a receipt is safe. Replaying an applied intent returns the
same semantic receipt without repeating the side effect.

## Versioning

Protocol version 1 uses additive compatible changes within a message version.
Removing or changing meaning requires a new schema version and a compatibility
window. Host and Cloud expose their supported version range during pairing and
heartbeat.

## Required contract tests

- forbidden-field canaries never appear in serialized payloads;
- stale projections cannot overwrite newer state;
- duplicate intents do not repeat side effects;
- expired and unknown intents fail closed;
- receipt retries are idempotent;
- host revocation blocks subsequent sync;
- local operation continues when every Cloud request fails.
