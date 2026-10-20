# Moxcrew product brief

Status: working product contract

Updated: 2026-10-20

## One-line promise

Moxcrew helps people turn goals into outcomes with AI agents, keeping the work,
context and decisions together in projects.

## Audience and project model

The owner selected a general-purpose product on 2026-10-19 under
[ADR 0038](decisions/0038-general-purpose-projects.md). Research, writing,
analysis, planning, personal work and software development are example uses;
developer status is not a product requirement.

A Project is a durable goal or body of work, with its context/resources, tasks,
agent work, results and accepted decisions. Project is the main user-facing
container; a separate Workspace hierarchy is not part of the selected basic
flow. A repository or working folder is a resource, not the project's identity.
Decisions include ordinary choices about a project's audience, evidence or
delivery, not only software architecture. Notes and generated suggestions remain
context until explicitly accepted. Decisions stay scoped to their project.

Chat starts with one message box and Send. Mox uses an available local connection
automatically; provider selection is an optional secondary override. An explicit
first Send can create its Project through an exact applied receipt. The resolved
provider plans the work; clarification replies and revisions use the same
composer while preserving the original goal and bounded prior context. Recipes,
work modes and roster configuration are optional details. Actual plans remain
inspectable and require explicit preparation and start before agents execute.
The bundled-guide recommendation remains available as a labelled alternative.
Files can be attached in the composer; targeted output selections return to the
same conversation. Optional Project tools contain brand guidance, taught skills,
bounded folder/URL triggers, parallel alternatives and inspectable source evidence.
Typed requests for images, video packages, static browser work and interactive
outputs enter this same plan/prepare/start flow. An armed trigger is explicit
time- and count-bounded delegation for its frozen recipe while the Host is awake.
It watches a source condition; it does not import an entire folder automatically.
See [eleven capabilities](state/eleven-product-capabilities.md) for engine and format limits.
Reusable rosters belong to Projects. Real agents exchange
attributed messages within the same Project and receive checked upstream files.
Closed agents do not automatically wake when sent a message. Single task remains
available for direct work. See [crew state](state/project-crew-orchestration.md).
Follow-ups can copy selected prior outputs; a side panel previews/exports actual
files. It offers forty inspectable recipes and eight bundled workflow guides.
Ordinary coding tasks, teams and their
Quick start guide keep Git worktrees. Recipes do not establish installed tools
or verified output quality. The owner selected one broad general-purpose ability
release, including presentations, video editing and research, under
[ADR 0039](decisions/0039-broad-general-purpose-ability-release.md). Evaluate
compatible existing skills and tools across documents, images, audio, data,
web and connected actions. Resource ingestion, tool integration and the full
ability paths still need bounded implementation and output verification.
See [conversation state](state/connected-project-conversation.md). Current internal
workspace/schema identifiers retain their meaning.

The owner accepted the broad plan as the
[1.0.0 release baseline](decisions/0040-version-one-release-baseline.md): eight
capability areas, forty abilities and ten composed workflows. The
[master release plan](plans/0039-version-one-release.md) defines observable
acceptance; [release state](state/version-one-release.md) distinguishes planned
scope from verified implementation. This does not change the current version.

## Jobs to be done

Across supported workflows, a customer needs to:

- describe a goal, relevant context and what a useful result looks like;
- inspect a proposed plan and the access required before work starts;
- review and use the result rather than only monitor agent activity;
- know what is running, waiting, failed, or finished;
- understand which provider, project, and work item owns each run;
- enqueue or revise work in the project rather than repeating its context;
- answer a blocking question and cancel unsafe or stale work;
- recover useful state after the UI or host restarts;
- check status and issue safe commands while away from the machine;
- keep source, credentials, prompts, and model subscriptions under local
  control.

## Product shape

### Provider connection and default experience

Mox works through an owner-connected local Codex, Gemini CLI or Claude Code
installation; one is sufficient. Provider installation, authentication and
task/tool readiness are separate onboarding checks. Moxcrew Cloud sign-in is
optional and does not connect those providers. The local application does not
imply on-device inference or bundled provider usage. Codex app-server and
supported Claude/Gemini headless adapters provide selected-provider execution;
live account/model access remains separately unverified. See
[provider execution state](state/provider-execution.md) and
[conversation-first state](state/conversation-first-orchestration.md).
The default connection choice uses existing readiness diagnostics rather than
asking the owner which engine fits the task. The real planner infers abilities
and crew members from the goal. See [automatic connection state](state/automatic-planning-provider.md).

The default skills must produce and revise checked outputs across the approved
release scope, with tool/access preflight and attributable inputs. A recipe
alone is not a delivered skill. Expressive Mox/crew motion should clarify real
connection, work and delivery states, with reduced motion and sparse idle
rendering. [ADR0042](decisions/0042-owner-providers-default-skills-and-motion.md)
records these owner requirements. [Task0041](plans/0041-connected-project-conversation.md)
implements the conversation, safe file follow-ups, explicit bounded connection
diagnostics and saved reduced motion. Bundled guides attach to real jobs; tool
availability and complete verified output paths remain release work.

Moxcrew has two products with a hard trust boundary:

### Moxcrew

The public native Rust/egui desktop application. It detects providers, normalizes their observable
state, stores canonical local events, presents the local console, and applies
validated commands. It works offline from Moxcrew Cloud.

### Moxcrew Cloud

The optional private SaaS. It authenticates users, stores sanitized operational
projections, and delivers allowlisted command intents to paired local hosts. In
the V1 contract the only remote intent is `cancel_run`. Cloud is never the
execution authority.

### Agent packs and teams

General-purpose Project crews retain the reusable roster separately from each
mission's goal, task attempts, outputs and messages. Teams is a primary native
navigation item, with the active crew also accessible from Chat.

Ready agent packs are inspectable local data, not executable plugins. In the
legacy coding implementation an owner selects a registered Git folder, an
orchestrator, roles, and limits. The local scheduler
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

The following is the implemented coding-control milestone and its release
evidence contract. It is not the limit of the general-purpose product direction
or proof that non-coding execution already works.

V0 is complete when one user can:

1. install Moxcrew locally;
2. see Claude Code, Codex, and Gemini CLI availability and active work in one
   normalized view;
3. create a local work item and send it through a supported provider;
4. observe durable lifecycle history and recover after restart;
5. pair the installation with Moxcrew Cloud through an outbound connection;
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
