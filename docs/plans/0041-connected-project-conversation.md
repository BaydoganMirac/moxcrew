# Task 0041: Connected Project conversation

Status: Bounded implementation complete; independently reviewed and locally verified

Updated: 2026-10-19

Owner authorization: “Onaylı başla” following ADR0042's provider connection,
default skill quality, conversational interaction and motion requirements.
This bounded implementation is part of the broad 1.0.0 baseline, not its
completion or a reduction of the forty-ability target.

## Outcome

A native Project conversation shows durable owner requests, honest work cards
and actual files. Owners prepare a goal, explicitly start supported work, open
files beside the conversation, and request a follow-up with selected prior
outputs copied into the new job. Provider setup distinguishes installed,
authentication and supported control. Expressive Mox motion follows real work
with a persistent reduced-motion control and no perpetual idle animation.

## Core contract

- `project_chat_send` accepts projectId, expectedProjectRevision, abilityId,
  title, instruction, optional replyToWorkItemId and referencePaths, plus the
  usual command envelope. Maximum eight references, selected from one terminal
  same-Project managed job. Reject references without a parent, wrong scope,
  links/traversal/missing/oversize files and active mutable parent work.
- Copy selected files into a fresh `inputs/` directory without changing their
  originals. Bound total input to 25 MiB and preserve exact copied bytes with
  a SHA-256 manifest. Derive parent/ability provenance from durable commands,
  not display names or prompt parsing. Inputs never become accepted decisions.
- Reuse existing queue, immutable Project binding, managed directory checks and
  provider start/cancel. V9 owner audit covers chat-send and desktop motion;
  V1/V3/V8 meanings
  stay fixed. Crash recovery returns a known durable inner queue receipt and
  never repeats an uncertain provider operation or filesystem copy.
- `project_chat_read` accepts projectId and optional offset/limit (default20,
  maximum50) and returns exact Project scope plus chronologically ordered
  entries: workItemId, abilityId, goal, replyToWorkItemId, status, revision,
  createdAt and runtimeIssue. The paged result excludes provider transcripts,
  credentials and result bodies. Reads are local, explicit and bounded.
- Earlier V8 Create jobs can appear as prior work with their exact recorded
  owner goal. A status card must not impersonate a generated assistant message.
  Existing scoped result reads/export display actual files in a side panel.

## Provider connection contract

`provider_connections_read` is an explicit local diagnostic read, never an
automatic login or inference call. It reports three allowlisted providers with
id, installed/version, authentication (`present`, `missing`, `unknown`), control
support and a safe reasonCode; no path, account identity or raw stdout/stderr.
Probe fixed argument lists with bounded time/output, kill/reap on timeout, do
not read token files or mutate user settings. Codex login-status exit semantics
and Claude auth-status JSON can establish saved credentials, not entitlement.
Gemini authentication remains unknown without a supported non-inference check.
Existing Codex control remains; Claude/Gemini control is not enabled speculatively.
Their implementation/proof is subsequent work within the accepted release.

## Native interaction and skills

- Chat history survives restart, project switching and provider loss. Preserve
  drafts and exact async receipt/job/file/export scope. Preparing never starts
  a provider silently. Provider setup opens only by deliberate action.
- Suggested skill routing may use explicit goal terms/parent skill; ambiguous
  goals ask for a type rather than silently selecting research. Manual override
  and inspectable forty-recipe catalog remain.
- Bundle eight self-authored, versioned skill guides/templates with concrete
  input/output/revision/verification rules. Attach the selected guide to real
  jobs. These are groundwork for complete tools, not forty verified renderers.
- Graphite/cobalt surfaces, compact conversation cards and on-demand artifacts
  stay native. Real activity drives bounded Mox animation; reduced motion is
  persistent. Idle must schedule no animation repaint. Do not invent progress,
  worker agents, files or generated responses.

## Ownership

- Runtime builder: creation.rs, new conversation.rs, tests/conversation.rs.
  Coordinate helper signatures before changing callers outside ownership.
- Provider builder: providers.rs plus new provider_connections.rs and related
  tests/fixtures only. No change to control capabilities or credentials.
- Interface builder: ui.rs, i18n.rs, create.rs and new chat.rs/connect.rs only.
- Coordinator: Host/domain/lib/store/bridge/main/preferences integration,
  bundled skill guides/metadata, motion module, docs and final artifacts.
- Independent reviewer: stable integrated commit, scope/recovery/privacy,
  meaningful checks and actionable native layout review.

## Verification and handoff

Real temporary output files and fake supported CLI processes prove queue,
follow-up bytes/hash, preserved originals, scope, bounds, stale revisions,
duplicate IDs, crash prefixes, restart and cancellation. Provider diagnostics
cover absent/unsupported/logged-out/malformed/timeout with secret canaries.
Cloud serializers must be unchanged by diagnostic/chat/skill private fields.
UI tests cover explicit actions, durable paging, file-panel scope, drafts,
minimum windows, TR/EN, themes and motion on/off. Verify actual native frames,
workspace checks, feature-off build and independent review before handoff.

No API key creation, provider login, global install, dependency download,
Cloud mutation/protocol/billing, publication or deployment is authorized here.
No live inference is needed for fixture checks. Record remaining provider,
tool-quality and platform evidence without declaring 1.0.0 complete.

## Acceptance evidence

Runtime `3fb10d5`, retry correction `26f0b26`, native UI `4e7b283` and
cross-skill follow-up/setup correction `4fcd06e` passed independent review.
The full workspace passed 331 checks with four explicit existing ignored gates;
the subsequent final desktop run passed all 126 checks. Formatting and strict
workspace/all-target/all-feature Clippy passed; the final UI correction also
passed strict desktop Clippy. Thirty-five final actual native GL frames cover
TR/EN, themes, normal/minimum sizes, file/connection panels and first use.
The feature-off macOS App/DMG passed signature, isolated launch/normal close,
writer-lock release and integrity checks. Full evidence and limits are in
[conversation state](../state/connected-project-conversation.md).

## Official diagnostic references

- [Codex login status](https://learn.chatgpt.com/docs/developer-commands?surface=cli)
- [Claude auth status](https://code.claude.com/docs/en/cli-reference)
- [Gemini headless](https://geminicli.com/docs/cli/headless/)
