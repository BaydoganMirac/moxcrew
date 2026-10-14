# Patchfleet architecture

Status: Native Rust desktop architecture

Updated: 2026-10-14

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
- the native Rust user interface;
- pairing credentials stored in the private local data file with owner-only
  permissions;
- payload sanitization before any network request;
- validation and application of remote command intents;
- the versioned public host-to-Cloud protocol.

## Private/Cloud responsibilities

Patchfleet Cloud owns in the Free beta release candidate:

- authenticated users and one personal workspace per user;
- host pairing and credential rotation;
- bounded sanitized host, provider, work, and run projections;
- durable `cancel_run` intents and their host receipts;
- authorization for every read and mutation.

Production authentication, normalized database storage, bounded retention,
export, and account deletion are implemented in the private Cloud release
candidate. Protocol V2 adds sanitized team supervision and typed decisions;
notifications, history, and billing consume only that sanitized Cloud state.

Cloud does not own provider credentials, repository contents, canonical run
events, local process control, or shell execution.

## Desktop deployable shape

`crates/patchfleet-core` owns domain validation, provider processes, canonical
storage, scheduling and outbound Cloud sync. `crates/patchfleet-desktop` renders
the interface in egui/eframe using a native window and Glow renderer. No WebView,
Node/Next server or embedded JavaScript bundle executes in the application.

A bounded async bridge calls the existing Host directly, preserves owner
command identities across retries, and keeps provider/network work off the
render thread. The core still validates every action. One OS writer lock owns
the data directory. Normal close awaits shutdown of owned provider processes.

The UI reacts to events rather than redrawing continuously. Active work polls
owned signals and scheduling every five seconds, while external provider probes
run at most once a minute or on explicit refresh. Idle status checks are sparse;
unpaired Cloud skips copying the event history. Canonical caches still scale
with history length, and startup replays the log.

[ADR 0024](decisions/0024-native-rust-desktop-ui.md) records the native interface
and lightweight priority. It supersedes the Tauri/React interface in ADR 0023,
while preserving the Rust core, data layout and trust boundary. Historical Node
modules remain event/protocol parity references.

## Core domain

The normalized model starts small:

- Host: one Patchfleet installation and its pairing state.
- ProviderInstallation: provider identity, availability, version, and
  capabilities.
- Workspace: an opaque local registration for one canonical Git worktree; its
  absolute path never enters the Cloud projection.
- Project: a stable local identity linking multiple registered workspaces.
- ProjectSource: revisioned owner context with identity and canonical provenance.
- AgentPack: a versioned declarative local role, capability, permission, limit,
  expected-output, and provenance contract.
- Team: immutable selected packs, limits, and one orchestrator for a workspace.
- TeamTask: one dependency-ordered unit with exactly one agent owner per attempt.
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

## Project memory boundary

Project is a stable local scope above registered workspaces, runs, and teams.
Several worktrees may share project memory while keeping execution files
separate. Slice A captures project messages and explicitly bound local work/team
sources through the canonical writer. V3 facts extend the log; strict V1/V2
operational contracts retain their meaning. Bindings freeze historical scope and
generated team prompts remain outside owner source capture. Removing a workspace
preserves its project's history. The native console requests bounded pages and
keyword filtering; ordinary snapshots carry only project metadata.

`project-memory.json` is a protected, rebuildable projection. It is excluded from
default Cloud serialization. Directly entered messages support append-only edits;
captured task/team facts remain immutable evidence. These records are context,
not accepted architecture decisions. ADR 0025 defines this implemented foundation.

The future decision/vector service belongs to the same native Rust host. Scoped
MCP tools or local service calls authenticate a run-bound project; agents cannot
select another project by supplying an ID or access raw database files.
Automatic extraction proposes records, while the owner accepts decisions.
Binding decision status is read from canonical facts rather than similarity
score. Memory text, queries, vectors, and tool credentials are local-only.

[ADR 0021](decisions/0021-project-scoped-local-decision-memory.md) defines the full
contract. Vectors, decision acceptance, agent tools and automatic extraction
remain later slices of Task 0022.

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

A single Rust writer with an exclusive OS file lock appends versioned JSON
events and fsyncs acknowledged changes. Derived JSON projections are written
atomically and rebuilt from events. The owning host caches accepted state;
unchanged UI reads do not replay history or rewrite caches. Unexpected log or
writer-lock replacement fails closed.

Private files reject symbolic links, reparse points and multiple hard links.
Unix permissions and protected Windows owner+SYSTEM ACLs precede private data
writes. V1 event meaning remains compatible; V2 local owner/remote-team audit
records add durable request and receipt recovery. Old provider ownership is
marked lost, and uncertain starts are never resent.

Workspace registration uses this same writer and command/receipt contract. The
desktop or offline CLI mutates the registry. The desktop uses a native OS folder
picker for registration and one-off work; paths are displayed read-only. A
bounded asynchronous preflight provides safe folder/access diagnostics, then
new registrations revalidate through the Host. Completed canonical registration
retries retain their original receipts rather than depending on current folder
eligibility. Work forms send an opaque local workspace ID, which the Rust host
resolves before creating a work item. Direct paths remain supported by the
offline CLI contract and do not change the Cloud protocol. OS consent does not
override filesystem permissions or provider sandboxing. See
[ADR 0028](decisions/0028-native-folder-consent-and-feedback.md).

Agent-pack installation and team lifecycle use the same single writer. Team
events project independently but ordinary task attempts enter the existing work
queue, so provider side effects retain the established idempotency, ownership,
cancel, and recovery rules. Pack instructions and team goals are local-only.

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
