# Project memory search

Status: Complete for approved scope; deployment/platform release work remains

Updated: 2026-10-15

[Task0032](../plans/0032-project-memory-search.md) extends the separate
[Memory protocol1](../memory-protocol.md) without changing V1–V5 canonical facts.
Only current accepted ADRs from the selected project are Local search results.
The existing Rust projection supplies weighted keyword ranking, free and offline.
It adds no database/model/Node runtime dependency to the desktop.

Cloud queries require an explicit Search action and selected shared project.
Typing stays on the device. The optional subscription service rechecks current
host, project, sharing and paid access before and after inference. Results have
bounded current IDs/revisions/titles, with clear semantic or keyword/index status.
Queries/results stay in ephemeral Bridge overlays and never enter canonical
events, snapshots, operational sync or a durable query cache. Scope, revision,
connection and owner-error retry guards reject stale results.

The TR/EN Cloud modal displays the original project, query-upload explanation,
index state and exact downloaded-copy action. A newer remote result requires
Sync before opening its exact version; the cached Local ADR is not overwritten.
Unavailable/partial semantic service explicitly falls back to keyword ranking.
Agents do not automatically consume these results yet.

## Evidence

- Local release benchmark at10/1000/10000 accepted synthetic ADRs measured p95
  0.093/4.073/37.335ms. SQLiteFTS5 was faster but adds a derived index and
  integration that this bounded Local workload does not require. See the
  [reproducible measurement](../benchmarks/local-retrieval-0.8.md).
- Real pinned Cloud encoder + HTTP + PostgreSQL/pgvector fixture:21/24 Top1,
  23/24 Top3,24/24 Top5 across TR/EN questions; a long-text query matched rank1.
  One Turkish question ranks4, so these small synthetic results are not a
  universal quality guarantee. No hosted model API or provider key is used.
- Two actual Rust Hosts verified pending-index keyword fallback, semantic search
  at a newer remote revision, exact download, replacement exclusion, lost ACK /
  restart, query canaries absent from durable files/tables, subscription/sharing
  denials and continued Local work through disposable loopback HTTP.
- Native macOS ARM64 frames show actual synthetic Host/model results:
  [Turkish](../screenshots/native-search-0.8-tr.png),
  [English800×560](../screenshots/native-search-0.8-en-min.png),
  [keyword fallback](../screenshots/native-search-fallback-0.8-tr.png) and
  [free Local](../screenshots/native-search-local-0.8-tr.png). These are real
  feature-only renderer captures, not a production OS-pixel capture claim.
- Independent core/backend/UI review found no remaining blocker; its CR input
  parity and stale protocol text findings were corrected. The actual capture
  also caught a smoke-startup modal reset, fixed with two regression checks.
- Final Rust all-feature workspace checks:179 passed,2 intentional manual/private
  fixture ignores. The real two-Host fixture was separately run and passed.
  Legacy reference149 checks, private46 unit checks and7 real PostgreSQL /
  upgrade checks passed. Strict Clippy, formatting and production Cloud build
  passed. A pre-existing test fixture used two clock reads for an exact5-minute
  lifetime; it now uses one instant to avoid a1ms over-limit race.

## Native development artifact

Version0.8.0 feature-off App and DMG were built on macOS ARM64. The executable is
12,464,128bytes; SHA256
`9a123622dba3b060584b37b1434ca83d12ff4e099d13a0309610f0cff2b7cc63`.
The DMG is6,398,816bytes (6.10MiB); SHA256
`6f1311910aff82b09647f4c932b8c3d598bd793bb13a4b0ced61de97d2981680`.
Ad-hoc signature, five folder-purpose keys, bundle/version, absence of smoke
controls and image integrity passed. Native OS launch used an explicit App path,
a disposable data directory and provider stubs: one1152×822 window, idle RSS
108432KiB (105.9MiB), CPU0.10s over the8s sample; normal close released the writer
lock and the process exited. OS window pixel capture remained unavailable;
the verified native renderer frames above are separate evidence. No installed
application or owner data was replaced. Public notarization and other native
platform runtime checks remain release work.

No production Cloud deployment/migration, existing-customer billing transition,
MCP memory tools or automatic ADR extraction is included. New server code and
model assets require a separately operated compatible Cloud release. Native
Windows/Linux runtime/signing evidence remains in the platform release matrix.

Final independent integration review accepted the pinned model, bounded offline
child lifecycle, cross-repository schema/limits and native query scope. Its one
additional process-startup teardown defect was fixed with an independently run
no-model regression. No unresolved blocker remains for the approved scope.
