# Scoped agent memory

Updated: 2026-10-15

Status: Complete for approved Task0034; independently reviewed and locally packaged. [Task0034](../plans/0034-scoped-agent-memory.md),
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
Generated role contracts are excluded from owner-source evidence. Role work
and failed-run retries inherit the team’s frozen project; unbound historical
teams stay unbound. A terminal failed start pauses automatic dispatch until
the owner fixes the connection and starts the queued task from Work. This
prevents unbounded provider retries while preserving canonical receipt semantics.
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
rollback uses the stopped pre-upgrade backup.


## Final verification and review

224 Rust workspace tests pass; four explicitly manual checks remain ignored in
that suite. The new installed-Codex check was also run explicitly and passed
against the final provider/Host/stdio code. The other ignored checks belong to
previous PostgreSQL/large-log milestones and were not rerun. All 149 legacy
protocol/parity tests pass. Workspace formatting and strict all-target,
all-feature Clippy pass. No dependency was added.

Independent review accepted the implementation with no remaining code blocker.
It closed immediate reader-side revocation (7c4d013), terminal-receipt V6
rejection (83ff392), canonical context ordering (668d8c1), frozen role/retry
scope and instruction normalization (c087af5), and bounded failed-start dispatch
with exact generated-role source exclusion (9f9913f). The remaining copy note
was fixed: preparation failures direct the owner to fix the connection and
start the still-queued task again, rather than claiming it needs queueing.
Tests cover newer run IDs that sort before older IDs and reader revocation
while consumer futures stay unpolled. No live model turn was used.

## Native 0.10.0 artifact

- DMG: `dist/native/Patchfleet_0.10.0_aarch64.dmg`, 6,658,356 bytes (6.35 MiB).
  SHA-256: `2d5896d28a17c5dcd919e7b426a254bc738f4e4f75986488b074d2656dd21c42`.
- Packaged executable: 12,962,848 bytes.
  SHA-256: `8a15b3d5e044c0e23123dd5a39a5c698bc3e48653051e7abde8f894c3894120b`.
- Release executable before packager stripping/signing: 13,020,448 bytes.
  SHA-256: `e8d420155ecb0969e65ebbb5415a7e8145cabfe2501f42b5b68910be2cb7449e`.

Feature-off App signature verification, version/bundle identifier, five macOS
folder-purpose keys, absence of smoke controls and DMG integrity pass. Native
framework dependencies contain no WebKit or Node runtime. The packaged stdio
mode itself passed initialize, three-tool inventory, safe invalid-ticket failure,
ping and normal exit without creating a data directory/writer or GUI. Its
static initialized RSS sample was 10,400 KiB (10.2 MiB); this excludes the
provider/model process and is not an active large-record measurement.

Explicit-path macOS launch used fresh temporary data and failing provider stubs:
PID80074, window1152×822, idle RSS108,848 KiB (106.3 MiB), accumulated CPU0.14s
after eight seconds. Normal close ended the process and released writer.lock.
OS window capture remained unavailable under host privacy permissions; the
separate actual OpenGL [native frame](../screenshots/native-decisions-0.10-tr.png)
was inspected. The TR/EN run-memory states were independently painted in UI
tests. These are macOS ARM64 development artifacts with ad hoc signing, not
publicly notarized releases. The installed Applications copy was not replaced.
No private Cloud code, deployment, global provider settings, credentials or
owner data were changed.
