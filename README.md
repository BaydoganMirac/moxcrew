# Patchfleet

Your coding agents. One command center.

Patchfleet is the public, local-first runtime and control console for Claude
Code, OpenAI Codex, Gemini CLI, and future coding agents. It keeps canonical
execution state on the developer machine and remains useful without a cloud
account.

Patchfleet Cloud is an optional private SaaS that receives only sanitized
operational projections and sends allowlisted command intents back to a paired
local installation.

## Current status

Patchfleet 0.2 includes durable local work control, declarative agent packs,
ready agent roles, bounded dependency-graph teams, and the optional paired
Cloud control plane. Local queues work, writes terminal receipts, starts and
cancels Codex inside an explicitly selected Git worktree, and keeps team goals,
task text, pack instructions, paths, prompts, and output on the machine. Claude
Code and Gemini CLI remain observation-only.

Cloud protocol V2 adds a sanitized team projection and typed, expiring team
actions. Local revalidates every action and may reject it; Cloud never receives
source or gains a generic shell.

Start with [the documentation map](docs/README.md) before changing code.

## Development

    npm install
    npm run dev

Release tarballs expose host lifecycle commands, workspace registration, and
declarative pack management. See the
[installation and recovery contract](docs/install.md).

Run `patchfleet doctor` for safe runtime, state, project, provider, and optional
Cloud diagnostics before changing configuration.

The local app runs at http://127.0.0.1:3000 and accepts only `localhost` or
`127.0.0.1` Host values. Use the manual refresh to store up to 20 recent
sessions per provider. Patchfleet does not retain prompts, titles, paths,
transcripts, tools, diffs, reasoning, or tokens in its observation projection.
Owner-authored work titles, instructions, and worktree paths are retained only
in the separate local work projection and never copied into command receipts.

Register a Git worktree once with `patchfleet workspace add .`, then choose the
project by name when queueing work. One-off absolute path entry remains under
the console's Advanced fallback. Enqueue works without Codex; start appears only after Codex is
observed as available with the tested app-server metadata on stable Codex
0.144.1 or newer. Patchfleet starts Codex with `workspace-write` and approval
policy `never`, and exposes cancel only for a linked active run.
Codex control belongs to the current local app boot. After an app restart,
Patchfleet hides Cancel until Refresh safely reconciles an old active run as
session-lost/blocked; it never launches replacement work automatically.

The Local console includes twelve built-in ready packs: Orchestrator, Product,
Design, Frontend, Backend, Full-stack, QA, Review, Security, Release, Docs, and
Research. Form a team by choosing one registered project, a bounded template,
agents, concurrency, retry count, time budget, approval gates, and failure
policy. Custom packs are strict JSON data:

    patchfleet agent-pack list
    patchfleet agent-pack install ./my-pack.json
    patchfleet agent-pack show pack:my-pack
    patchfleet agent-pack remove pack:my-pack

Packs cannot load executable code or widen the Codex sandbox.

Cloud remains optional. Create a short-lived pairing code in Patchfleet Cloud,
then enter its URL, a host name, and the code in the local Cloud panel. The
local launcher syncs outbound in the background. It sends only current opaque
IDs, states, revisions, provider capabilities, versions, and coarse timestamps;
local work text and worktree paths stay on the host. V2 may also send anonymous
workspace aliases, installed pack IDs and roles, and sanitized team/agent/task
states. Owner-authored bounded goals, answers, and notes travel only inside the
specific typed intent and are never copied into the projection. Disconnecting
or a Cloud outage does not disable the local console.

Gemini lifecycle observation is opt-in. From the Patchfleet checkout, let
Gemini CLI link the checked-in extension, then restart Gemini:

    gemini extensions link extensions/patchfleet-gemini
    gemini extensions list --output-format json

Future `SessionStart`, `BeforeAgent`, and `AfterAgent` hooks enter the console
on manual refresh. Gemini CLI owns consent and extension state; Patchfleet does
not edit Gemini settings. Remove the extension with:

    gemini extensions uninstall patchfleet-gemini

To verify the boundary:

    npm test
    npm run build
    npm run start

## Product boundaries

- Local execution is canonical.
- Cloud is optional and cannot execute shell commands.
- Model subscriptions and credentials remain user-owned and local.
- Source code, diffs, prompts, transcripts, paths, and secrets are not uploaded
  by default.
- Provider-specific behavior stays behind small adapters.

## License

Patchfleet is licensed under the [Apache License 2.0](LICENSE).

Report vulnerabilities privately through the [security policy](SECURITY.md).
