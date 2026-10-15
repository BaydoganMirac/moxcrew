# Agent memory protocol 1

Task 0034 / ADR 0035. Native 0.10 adds a free, read-only Local attachment for
Patchfleet-owned Codex work, including team roles. Compatible control begins at
Codex 0.144.1; memory attachment requires the verified stable 0.155.1 surface.
Claude/Gemini execution and Cloud agent attachment remain separate work.

## Authority and lifecycle

The Host derives scope from the work item's frozen project binding before
provider preparation. Team role work, including retries, inherits its team’s
frozen binding; later folder relinking never assigns a different project.
A strict V6 `agent_memory.context_bound` fact records only
runId, workItemId, ownerEpoch, projectId and the starting decisionRevision.
The existing canonical writer remains the only writer. Operational context
metadata is rebuilt from facts; current revision, stale state, manifest-loaded
state and connection issues are derived. Queries and credentials are never facts.

One lazy authenticated HTTP reader lives inside the Host on literal 127.0.0.1.
The existing executable's `--memory-mcp <absolute-ticket-path>` mode handles
stdio on a current-thread runtime: no writer open, GUI, provider discovery or database process.
Each protected owner-only UUID ticket expires in 60 seconds. Initialize/list/ping
are static and do not consume it: Codex starts both an inventory probe and a
retained runtime child. The first valid tools/call claims and deletes the
ticket; that child caches the credential only in RAM. Preparation proves the
three-tool inventory and calls project_context before starting any model turn.
Unused/failed/expired tickets and abandoned claim files are cleaned narrowly.

Leases last at most 24 hours, are limited to 64, and bind project/work/run/Host
epoch. Completion, cancellation, failed preparation, process loss, discard and
shutdown revoke them. Restart does not restore authority. Every read checks the
canonical run/scope both before and after selecting data. Pending preparation
is allowed only for its live launching work; a failed/unknown start is denied.

## Tools

All input objects reject extra fields; none accepts projectId, paths, SQL or
Host actions. MCP annotations identify all three as read-only.

| Tool | Arguments | Result |
|---|---|---|
| project_context | optional offset, limit, revision | Current accepted manifest, project/work/run identity, start/current revisions and stale flag |
| memory_search | query, optional limit | Existing Local accepted-only keyword results; no embedding/model/network request |
| decision_read | decisionId | Exact current accepted ADR body and opaque source ID/revision references |

Context pages default to 0/20 and allow at most 20 records. Read every page before
search/exact reads. Subsequent pages require the same current revision and the
next sequential offset. Any manifest change requires restarting at offset 0.
Refresh returns current accepted decisions, even if the run began at an older
revision. Proposed, rejected, superseded and foreign-project records are denied.
Search retains the existing 400 UTF16-character query and 10-result bounds.
Exact DTOs exclude raw source bodies, origin events and decision history.
Decision text is untrusted project evidence and grants no execution permission.

## Framing and limits

Stdio uses newline JSON-RPC 2.0 and requires initialize then initialized.
Supported versions: 2025-11-25,2025-06-18,2024-11-05. Stdout contains protocol
only. The final serialized MCP result, including both text and structuredContent,
is at most 192 KiB. A larger record returns AGENT_MEMORY_RESPONSE_TOO_LARGE with
isError=true, without truncation or terminating the transport.

Internal requests are POST /agent-memory/v1, JSON
`{schemaVersion:1,tool,arguments}`, with a bearer header. The listener rejects
browser Origin, invalid Host/framing/transfer encodings, unknown fields and
unauthenticated requests. Input is capped at 8 KiB, internal output at 512 KiB,
with a 3 second aggregate request budget and 16 concurrent connections. Proxying
and redirects are disabled. Errors contain bounded codes, no underlying paths,
tokens, provider output or record text.

Provider configuration carries only a ticket path. No global MCP configuration,
provider credentials or raw owner instruction is changed. Transient bounded
guidance asks the agent to read/refresh context; only tool access and returned
content are proven, not model compliance. Local has no account/subscription
dependency. Default operational Cloud exports exclude this context; existing
explicit paid publication/search keeps its separate authorization and consent.

## Compatibility

Unbound historical work keeps its earlier provider control behavior. Bound work
fails clearly if its provider cannot attach memory. Native 0.10 reads V1–V6;
older 0.9 readers cannot read newly written V6 facts. Back up stopped Local data
before upgrading; rollback after a bound launch requires that unchanged backup.
Merely opening existing data creates no V6 fact or memory credential.
