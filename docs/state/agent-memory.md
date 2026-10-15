# Scoped agent memory

Updated: 2026-10-15

Status: Implementation complete; independent review and final package verification
in progress. [Task0034](../plans/0034-scoped-agent-memory.md),
[ADR0035](../decisions/0035-run-scoped-agent-memory.md),
[protocol](../agent-memory-protocol.md).

## Implemented

Owned project-bound Codex work and team roles get project_context, memory_search
and decision_read automatically through per-thread MCP configuration. Frozen
work scope controls access. Current accepted Local ADRs only; exact DTOs omit
raw source text/history/origins. Full paginated context precedes exact/search
reads; changed manifests require refresh. The native task card shows starting
and current revisions, manifest-read status and actionable connection failures
in Turkish/English. Owner content is unchanged.

The existing Rust Host owns a lazy loopback reader and canonical writer. Existing
CLI/desktop executables have a stdio-only early mode. A protected 60-second
one-shot ticket is claimed only on the first valid tool call, then deleted;
credentials remain in RAM. Initialization/inventory probes do not consume the
ticket. Preparation performs a real context preflight before any provider turn.
Leases bind project/work/run/epoch, expire within 24 hours and revoke on terminal,
cancel, process loss, discard, prepare failure and shutdown. Strict V6 context
facts contain no credential, query or record body. Old unbound work stays usable.
No production dependency, separate database, global provider config or Cloud
authorization change was added. Local remains free and account-independent.

## Evidence

Provider commit 1e186c4: 12 provider checks, including per-thread attachment,
preflight failure and synchronous lifecycle revocation. Transport commit 049f237:
10 checks, including actual stdio children, inventory/runtime ticket behavior,
expiry, framing, link/mode rejection, interrupted-claim cleanup and final
192 KiB serialized MCP size errors. Internal RPC remains bounded at 512 KiB.

Coordinator tests exercise two actual linked Git worktrees against one real Host,
accepted-only exact reads, pagination/refresh, foreign project rejection, raw
source/query/token canaries, operational Cloud exclusion, pending preparation
without operations-mutex deadlock, empty manifest and canonical start failure.
An explicit ignored check also passed against installed Codex 0.155.1, the real
Rust CLI stdio bridge and real Host from both worktrees: tickets consumed,
current manifest read, exact ADR available, shutdown revoked both leases.
The provider process used disposable HOME/CODEX_HOME and an empty inherited
environment. No model turn, credential read or owner provider setting change.

## Remaining

Automatic ADR drafting and Cloud agent attachment are separate slices. Real
model compliance is not established by a transport preflight. Claude/Gemini
execution, Windows/Linux native runs, public signing/notarization and deployment
remain unverified. Native 0.10 is required after a V6 bound-launch fact is written;
rollback uses the stopped pre-upgrade backup. Final review, complete test totals,
native artifact hashes and launch measurements are recorded below when verified.
