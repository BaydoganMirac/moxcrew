# Patchfleet product brief

Status: working product contract

Updated: 2026-10-15

## One-line promise

Patchfleet gives serious builders one local command center to form, direct,
supervise, and recover teams of coding agents, with an optional Cloud view for
safe remote decisions.

## Jobs to be done

When several coding agents are active, the customer needs to:

- know what is running, waiting, failed, or finished;
- understand which provider, project, and work item owns each run;
- enqueue or revise work without switching between terminal windows;
- answer a blocking question and cancel unsafe or stale work;
- recover useful state after the UI or host restarts;
- check status and issue safe commands while away from the machine;
- keep source, credentials, prompts, and model subscriptions under local
  control.

## Product shape

Patchfleet has two products with a hard trust boundary:

### Patchfleet

The public native Rust/egui desktop application. It detects providers, normalizes their observable
state, stores canonical local events, presents the local console, and applies
validated commands. It works offline from Patchfleet Cloud.

### Patchfleet Cloud

The optional private SaaS. It authenticates users, stores sanitized operational
projections, and delivers allowlisted command intents to paired local hosts. In
the V1 contract the only remote intent is `cancel_run`. Cloud is never the
execution authority.

### Agent packs and teams

Ready agent packs are inspectable local data, not executable plugins. An owner
selects a workspace, an orchestrator, roles, and limits. The local scheduler
coordinates bounded provider runs and keeps task ownership, decisions, retries,
questions, and receipts durable. Cloud receives only the sanitized operational
projection and allowlisted expiring intents.

## Product and subscription boundary

Local is free and works without an account or subscription. Local projects,
agent teams and the planned local decision/search/ADR capabilities remain in
that free product. Users supply their own coding-provider access and pay any
provider costs independently.

Cloud is an optional subscription service for hosted and cross-device value:
remote supervision, notifications, sync and explicitly shared project memory
as each capability is implemented. There is no permanent Cloud Free tier in
the new product model. Pricing, allowances and any trial are not finalized.
Existing Cloud Free/Solo behavior has not yet been migrated to this model.
Cancellation never disables Local or removes downloaded local data. See
[ADR 0029](decisions/0029-free-local-and-subscription-cloud.md).

## Project memory

The project/source foundation gives a project a stable identity across worktrees.
Owners explicitly link registered workspaces, enter project messages, and inspect
locally captured task instructions, team goals, owner answers and approval notes.
Message edits retain revision history; changing a worktree link does not move old
sources into another project. Keyword filtering and bounded native pages keep
reads scoped. Sources remain context; owners create sourced ADR drafts separately.

The Decisions view supports proposed, accepted, rejected and superseded ADRs.
Only explicit owner acceptance makes a decision authoritative. Accepted text
and source versions are immutable; replacing a decision preserves its history.
This free Local feature requires no account, inference or embedding service.
Owned Codex work and team roles now read current accepted Local decisions through
three scoped read-only MCP tools. Worktree links share one project memory; the
agent cannot select another project or read raw source bodies. Local keyword
retrieval stays free; Cloud vector retrieval retains its paid boundary. Automatic
source-backed ADR drafts are optional per project with explicit provider-upload/usage consent; they remain proposed until owner acceptance. Existing sources require deliberate queueing. Agent Cloud attachment remains a later slice. See
[Task 0022](plans/0022-project-memory-and-decision-service.md). External agent
conversations require an explicit import; this foundation does not import them.
Memory does not widen the default Cloud data boundary. The implemented optional
subscription sharing service requires explicit project creation/linkage and
per-ADR preview and confirmation under [Memory protocol1](memory-protocol.md).
One paired installation publishes accepted decisions; explicitly linked readers
in the same personal workspace download immutable exact versions. Downloaded
copies do not become Local accepted decisions. Payment and ordinary pairing
alone do not upload memory. Access loss keeps downloaded copies visibly stale;
sharing off stops further access, without erasing copies already downloaded.
Service deployment and the broader existing-customer commercial transition
remain separate release work.

## V0 promise

V0 is complete when one user can:

1. install Patchfleet locally;
2. see Claude Code, Codex, and Gemini CLI availability and active work in one
   normalized view;
3. create a local work item and send it through a supported provider;
4. observe durable lifecycle history and recover after restart;
5. pair the installation with Patchfleet Cloud through an outbound connection;
6. view sanitized status remotely;
7. issue the safe V1 remote cancel action and receive a definitive receipt.

macOS is the development reference platform. Native Windows and Linux build
and runtime evidence are desktop release gates. The CI matrix includes macOS
ARM64/Intel, Windows x64, and Linux x64/ARM64 targets.

## Explicit non-goals for V0

- reselling model tokens or subscriptions;
- a general-purpose workflow engine;
- unbounded autonomous planning or permanent background agents;
- cloud execution or hosted coding sandboxes;
- arbitrary shell access from a browser;
- uploading repositories, diffs, raw prompts, transcripts, or terminal output;
- mobile-native applications;
- organization SSO, enterprise policy, or self-hosted Cloud;
- plugin marketplaces or a generic provider SDK.

## Product principles

- Local-first is a trust promise, not a cache strategy.
- Show operational truth before adding automation.
- Safe remote control beats broad remote control.
- A provider abstraction must preserve useful differences, not hide them.
- Cloud value must not make the local application intentionally incomplete.
- The smallest complete control loop is more valuable than a broad mockup.

## Success criteria

Before a public V0 launch:

- all three providers pass the same observation and lifecycle conformance suite;
- a local restart does not lose acknowledged work or terminal run state;
- remote commands are idempotent and always end in applied, rejected, expired,
  or failed;
- redaction tests prove forbidden data is absent from default Cloud payloads;
- a new user reaches the first unified agent view in under ten minutes;
- the local application remains fully usable when Cloud is unavailable;
- macOS and Windows installation paths are documented and tested.
