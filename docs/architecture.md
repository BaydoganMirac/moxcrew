# Patchfleet architecture

Status: V0 target architecture

Updated: 2026-07-16

## System boundary

Patchfleet uses a local execution plane and an optional remote control plane.
The boundary is intentionally asymmetric: the local host can connect outbound
to Cloud, but Cloud cannot open an inbound session to the developer machine.

    Provider CLIs
      Claude Code | Codex | Gemini CLI
                |
          provider adapters
                |
      local runtime and event writer
          |                 |
    local projection     command validator
          |                 |
       local UI        local side effects
          |
      sanitized projector
                |
         outbound HTTPS
                |
        Patchfleet Cloud
     remote UI + intent queue

## Public/local responsibilities

The public repository owns:

- provider discovery and capability detection;
- provider-specific observation and execution adapters;
- normalized hosts, agents, work items, runs, questions, and receipts;
- one durable local event writer;
- rebuildable local projections;
- the local web user interface;
- pairing credentials stored in the private local data file with owner-only
  permissions;
- payload sanitization before any network request;
- validation and application of remote command intents;
- the versioned public host-to-Cloud protocol.

## Private/Cloud responsibilities

Patchfleet Cloud owns in the Phase 3 private alpha:

- one owner session and one fixed workspace;
- host pairing and credential rotation;
- bounded sanitized host, provider, work, and run projections;
- durable `cancel_run` intents and their host receipts;
- authorization for every read and mutation.

Production authentication, database storage, notifications, billing, history,
and retention jobs remain future Cloud responsibilities, not implemented V1
features.

Cloud does not own provider credentials, repository contents, canonical run
events, local process control, or shell execution.

## V0 deployable shape

V0 is one local Node.js/Next.js application. UI routes, server-side runtime
modules, adapters, storage, and Cloud sync are separate code boundaries inside
one deployable process.

Do not create microservices or a desktop wrapper before the control loop works.
A background host process may be extracted later when reliable boot-time and
headless operation require it. A desktop shell may be added later for packaging
and OS integration without moving canonical state into the shell.

## Core domain

The normalized model starts small:

- Host: one Patchfleet installation and its pairing state.
- ProviderInstallation: provider identity, availability, version, and
  capabilities.
- WorkItem: owner intent and ordering independent of a provider session.
- Run: one execution attempt for a work item.
- AgentSession: provider-observed execution context associated with a run when
  possible.
- Question: a blocking request that needs an owner answer.
- CommandIntent: a requested state transition from local UI or Cloud.
- CommandReceipt: the terminal result of applying an intent.
- Event: immutable local fact used to rebuild projections.

Provider-native fields may be retained in a local namespaced extension. They do
not enter the shared contract or Cloud projection until a concrete product use
case and data classification exist.

## Provider adapter boundary

The first read-only slice needs only:

- probe: determine whether the provider is available and what capabilities can
  be observed;
- observe: return normalized current sessions and lifecycle state.

Phase 2 adds bounded same-boot start/cancel control for compatible stable Codex
versions. Claude Code and Gemini CLI remain observation-only. The shared
interface must not pretend every provider supports pause, resume, follow-up, or
structured questions. Each adapter reports a capability map, and the UI
exposes only proven actions.

Codex is implemented first because the existing Agent Console provides the most
reusable evidence for its lifecycle. Claude Code and Gemini CLI follow against
the same conformance cases. V0 is not provider-complete until all three pass.

## Local persistence

A single writer appends versioned JSON events to a local log using Node.js file
primitives. Derived JSON projections are written atomically and may always be
rebuilt from events.

V0 recovery rules:

- reject malformed events except for an incomplete final line caused by a
  crash;
- keep event identifiers and command idempotency keys unique;
- write a terminal receipt for every accepted command;
- never infer that a side effect succeeded merely because it was requested;
- compact only after a verified snapshot and keep an auditable boundary.

If concurrency, query volume, or migration pressure proves this inadequate,
SQLite may replace the storage implementation behind the same event contract.
It is not a V0 dependency.

## Read and write paths

Local observation:

1. an adapter observes a provider;
2. the runtime validates and normalizes the observation;
3. changed facts become local events;
4. the projector rebuilds the user-facing view;
5. the local UI reads only projections.

Remote action:

1. Cloud authorizes an owner request and records an intent;
2. the paired host retrieves the intent over outbound HTTPS;
3. the local validator checks version, identity, expiry, capability, revision,
   and idempotency;
4. the local adapter attempts the side effect;
5. a local event and terminal receipt are persisted;
6. the sanitized receipt returns to Cloud.

## Failure posture

- Cloud unavailable: local operation continues and sync retries with bounds.
- Provider unavailable: its adapter reports unavailable without taking down
  other providers.
- UI restart: durable state is rebuilt from local events.
- Duplicate intent: return the original receipt; do not repeat the side effect.
- Unknown or expired intent: reject and record the reason.
- Schema mismatch: fail closed and instruct the user to upgrade.
- Sanitizer failure: send nothing.

## Security posture

- Pairing secrets never appear in logs or projections.
- The host authenticates every Cloud response and intent.
- Cloud authorization is scoped by user, workspace, and host.
- Remote commands are an enum with validated payloads, never strings executed
  by a shell.
- Local provider credentials are discovered through provider-supported local
  mechanisms and are never copied into Patchfleet Cloud.
- Telemetry is opt-in and must use the same data-classification rules as Cloud
  sync.
