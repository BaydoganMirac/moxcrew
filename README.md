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

Phase 1 observation, Phase 2 durable local work control, and the Phase 3 paired
Cloud control slice are complete. The local app queues work, writes safe
terminal receipts, starts/cancels Codex inside an explicitly selected Git
worktree, and can publish a bounded sanitized operational view to Patchfleet
Cloud. Claude Code and Gemini CLI remain observation-only. The only remote
command in protocol version 1 is `cancel_run`.

Start with [the documentation map](docs/README.md) before changing code.

## Development

    npm install
    npm run dev

The local app runs at http://127.0.0.1:3000 and accepts only `localhost` or
`127.0.0.1` Host values. Use the manual refresh to store up to 20 recent
sessions per provider. Patchfleet does not retain prompts, titles, paths,
transcripts, tools, diffs, reasoning, or tokens in its observation projection.
Owner-authored work titles, instructions, and worktree paths are retained only
in the separate local work projection and never copied into command receipts.

Queue work from the console by providing a title, instruction, and absolute Git
worktree root. Enqueue works without Codex; start appears only after Codex is
observed as available with the tested app-server metadata on stable Codex
0.144.1 or newer. Patchfleet starts Codex with `workspace-write` and approval
policy `never`, and exposes cancel only for a linked active run.
Codex control belongs to the current local app boot. After an app restart,
Patchfleet hides Cancel until Refresh safely reconciles an old active run as
session-lost/blocked; it never launches replacement work automatically.

Cloud remains optional. Create a short-lived pairing code in Patchfleet Cloud,
then enter its URL, a host name, and the code in the local Cloud panel. The
local launcher syncs outbound in the background. It sends only current opaque
IDs, states, revisions, provider capabilities, versions, and coarse timestamps;
local work text and worktree paths stay on the host. Disconnecting or a Cloud
outage does not disable the local console.

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
