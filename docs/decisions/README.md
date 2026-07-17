# Architecture decision records

Accepted records are append-only. If a decision changes, add a new ADR that
supersedes the old one.

| ADR | Status | Decision |
|---|---|---|
| [0001](0001-local-first-public-runtime-private-cloud.md) | Accepted | Public local runtime with optional private Cloud |
| [0002](0002-provider-adapters.md) | Accepted | Small capability-aware provider adapters |
| [0003](0003-sanitized-projections-and-durable-intents.md) | Accepted | Sanitized outbound projections and durable inbound intents |
| [0004](0004-append-only-local-events.md) | Accepted | Append-only local events with derived projections |
| [0005](0005-single-node-web-deployable-first.md) | Accepted | One Node/Next.js deployable before daemon or desktop extraction |
| [0006](0006-governance-before-agent-runtime.md) | Accepted | Establish shared rules and a small development team before runtime implementation |
| [0007](0007-public-and-internal-document-boundary.md) | Accepted | Keep public technical trust contracts separate from internal product and Cloud context |
| [0008](0008-browser-originated-local-request-boundary.md) | Accepted | Protect the local browser surface without a custom raw HTTP server |
| [0009](0009-codex-observation-uses-supported-app-server-metadata.md) | Accepted | Observe only supported Codex lifecycle metadata and preserve unknown state |
| [0010](0010-supported-provider-observation-surfaces.md) | Accepted | Use supported Claude JSON and opt-in Gemini hooks behind a minimal lifecycle ingress |
| [0011](0011-multi-provider-observation-projection.md) | Accepted | Project the three provider observations together before adding Gemini hook ingress |
| [0012](0012-gemini-native-extension-lifecycle-inbox.md) | Accepted | Use Gemini's native extension lifecycle and a sanitized inbox before the single writer |
| [0013](0013-local-work-intake-shares-the-canonical-event-log.md) | Accepted | Keep work intake and command receipts in the canonical local event log |
| [0014](0014-codex-control-uses-a-bounded-app-server-session.md) | Accepted | Start and cancel Codex through a bounded app-server session |
| [0015](0015-phase-3-uses-outbound-polling-and-remote-cancel.md) | Accepted | Pair outbound and prove one sanitized remote-cancel loop |
| [0016](0016-package-the-node-host-before-a-desktop-shell.md) | Accepted | Package the Node host behind an npm CLI before adding a desktop shell |
| [0017](0017-use-a-local-workspace-registry-before-cloud-aliases.md) | Accepted | Register local Git worktrees before adding Cloud aliases |
| [0018](0018-support-node-22-and-24-for-the-public-beta.md) | Accepted | Support Node 22 and verify Node 24 compatibility |
