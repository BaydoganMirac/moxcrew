# Task0034 Scoped agent memory

Status: Approved implementation in progress

Owner approval:2026-10-15, “Yap! Onaylı go seriden çıkar”, for the proposed
project-bound read-only MCP tools and two-worktree verification.

## Outcome

Owned Codex work and team roles in different linked worktrees can retrieve the
same current owner-accepted Local ADRs through three bounded tools. Scope comes
from the actual frozen work/project binding, never model-supplied paths or IDs.
Local stays free, offline from Cloud and account-independent. Existing paid
Cloud publication/search authorization remains unchanged; this first agent
attachment reads Local accepted decisions. Automatic drafting, writes, external
conversation imports, other-provider execution and hosted deployment are separate.

## Contract

- Tools: project_context, memory_search, decision_read. No acceptance/mutation,
  source-file access, SQL, arbitrary Host actions or project-selection argument.
- A lazy authenticated loopback read service lives inside the existing Host.
  A thin --memory-mcp stdio mode in the existing executable proxies only these
  tools. It never opens a writer or database, starts providers or renders a UI.
- One private, expiring, single-use bootstrap ticket transfers an opaque bearer
  to the child. Provider config/arguments contain only the ticket path, never
  the bearer. Use existing owner-only storage/link protections; remove consumed
  tickets and clean failed/stale tickets. No secret, query or ADR body enters
  operational snapshots, canonical context facts, prompts as credentials, exports
  or Cloud. Protocol output has no application logging on stdout.
- RAM leases bind project/work/run/host epoch, expire within24h and revoke on
  terminal/cancel/lost process, failed prepare and shutdown. Every request checks
  the live canonical scope/run before and after reads. Restart grants no access.
- MCP uses bounded newline JSON-RPC, initialized handshake, exact schemas and
  three read-only tools; initialize supports2025-11-25/2025-06-18/2024-11-05.
  Internal RPC is POST /agent-memory/v1 on literal127.0.0.1 only, bearer header,
  exact fields {schemaVersion:1,tool,arguments}, no browser Origin, no redirects,
  at most8KiB input/512KiB internal output and a3s aggregate budget.
  The final MCP JSON response, including text/structuredContent duplication, is
  capped at192KiB with an explicit safe tool error on overflow. Local commands stay
  responsive. Oversize exact records fail explicitly, without truncation.
- project_context accepts optional offset/limit/revision; default0/20, max20.
  Subsequent pages require the same revision. It returns the complete paginated
  current accepted manifest, project identity and start/current memory revision.
  Context changes are stale and require a fresh first page; no silent omission.
- memory_search accepts query and optional limit only (existing400UTF16/10
  bounds). Reuse free keyword search, accepted-only. decision_read accepts only
  decisionId and returns an accepted exact DTO with immutable body and opaque
  source ID/revision references, excluding raw source bodies and origin events.
- Record only run/work/project/epoch and the starting decision manifest revision
  through one strict V6 agent_memory.context_bound fact before provider prepare.
  No tool invocation/query/body facts. Preserve V1–V5 and frozen historical work
  bindings. Derived run memory metadata reports stale context without rewriting
  an active provider turn. Old work lacking a project remains compatible.
- Provider attachment uses verified thread/start config with the three enabled
  tools and required startup. Prove connected/tool inventory before turn/start.
  The first verified memory-capable Codex is0.155.1; keep older compatible
  control usable for unbound work and fail memory-bound work explicitly if
  attachment is unsupported. No global provider config or credentials change.

## Ownership

- Coordinator: task/ADR/protocol/state, lib.rs/domain.rs/store.rs/host.rs/work.rs,
  main/CLI entry, host_memory_tests.rs, teams.rs, ui.rs/i18n.rs, Cargo/version/package, integration/native proof and final handoff.
- Memory builder: new agent_memory.rs (bounded lease/RPC/stdIO bridge and V6
  context projection helpers), tests/agent_memory.rs only. No Host/provider/store
  edits; call coordinator's Host boundary with the agreed binding type.
- Provider builder: providers.rs and tests/provider_memory.rs only. Add optional
  MemoryConfig{config,active}, prepare_with_memory, attachment inventory check and
  synchronous lifecycle revocation. Existing prepare stays compatible.
- Independent reviewer: stable commits only; scope, replay, secret transport,
  expiry/revocation, pending launches, provider compatibility and single writer.

## Acceptance

Real stdio MCP initialize/list/call through the running Host; two actual worktree
bindings read one accepted ADR; another project, unknown IDs, proposed/rejected/
superseded records, forged args, wrong bearer, expired/revoked/completed run and
restart are denied. Changed manifests require refresh; no source/query/token
canaries enter files/events/default Cloud payloads. Prove no extra writer and
continued Local commands while MCP waits. Exercise installed Codex's real MCP
handshake/tool call in isolated temporary provider state without an inference
turn or credential reads. Run affected checks, independent review and feature-off
native package checks. Real model adherence and other native OS runs are stated
separately from the proven transport.
