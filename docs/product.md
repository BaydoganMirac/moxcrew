# Patchfleet product brief

Status: working product contract

Updated: 2026-07-16

## One-line promise

Patchfleet gives serious builders one local command center to see, direct, and
recover work running across Claude Code, Codex, and Gemini CLI, with an
optional low-cost Cloud view for remote access.

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

The public local application. It detects providers, normalizes their observable
state, stores canonical local events, presents the local console, and applies
validated commands. It works offline from Patchfleet Cloud.

### Patchfleet Cloud

The optional private SaaS. It authenticates users, stores sanitized operational
projections, and delivers allowlisted command intents to paired local hosts. In
the V1 contract the only remote intent is `cancel_run`. Cloud is never the
execution authority.

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

macOS is the development reference platform. Windows support is a V0 release
gate, not a requirement for the first vertical slice.

## Explicit non-goals for V0

- reselling model tokens or subscriptions;
- a general-purpose workflow engine;
- autonomous team planning with dozens of permanent agents;
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
