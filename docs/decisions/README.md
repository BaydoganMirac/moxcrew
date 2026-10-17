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
| [0019](0019-declarative-packs-and-local-team-orchestration.md) | Accepted | Use declarative local agent packs and a bounded local team orchestrator |
| [0020](0020-capability-negotiated-team-protocol-v2.md) | Accepted | Add capability-negotiated sanitized Cloud team supervision |
| [0021](0021-project-scoped-local-decision-memory.md) | Accepted product contract | Project-scoped local memory with sourced ADR drafts and explicit owner acceptance |
| [0022](0022-rust-local-host-with-a-web-ui.md) | Proposed | Evaluate a Rust local host while retaining the web UI and Cloud stack |
| [0023](0023-rust-tauri-desktop-runtime.md) | Accepted | Rust host and Tauri desktop supersede earlier Node deployment assumptions |
| [0024](0024-native-rust-desktop-ui.md) | Accepted | Native Rust/egui interface supersedes Tauri; prioritize low RAM and event-driven rendering |
| [0025](0025-project-source-memory-foundation.md) | Accepted | Stable projects, explicit worktree links and durable owner sources before decisions or vectors |
| [0026](0026-native-desktop-language-preferences.md) | Accepted | Turkish/English native copy with protected owner language preference; source content stays original |
| [0028](0028-native-folder-consent-and-feedback.md) | Accepted | Native folder selection and actionable access feedback |
| [0029](0029-free-local-and-subscription-cloud.md) | Accepted product boundary | Free Local and optional subscription Cloud; runtime transition remains separate |
| [0030](0030-shared-decision-publication.md) | Proposed | One publishing host per shared project; owner-accepted local facts and Cloud published revisions |
| [0031](0031-versioned-local-decision-events.md) | Accepted | Version4 sourced Local decision lifecycle |
| [0032](0032-opt-in-shared-memory-transport.md) | Accepted | Explicit accepted-only publication and exact shared reads |
| [0033](0033-versioned-project-memory-retrieval.md) | Accepted | Ephemeral project search, measured Cloud vectors and free Local keyword retrieval |
| [0034](0034-native-browser-account-sign-in.md) | Accepted | Optional native account sign-in through external browser, strict loopback/S256 and existing installation credential |
| [0035](0035-run-scoped-agent-memory.md) | Accepted | Run-bound read-only Local MCP access with single-use bootstrap and revocable leases |
| [0036](0036-consented-provider-draft-extraction.md) | Accepted | Per-project consented provider drafting, bounded V7 jobs and attributed V4 proposals; explicit owner acceptance |
| [0037](0037-dobbly-display-identity.md) | Proposed implementation boundary | Dobbly visible identity with stable installation, data and protocol identifiers |
