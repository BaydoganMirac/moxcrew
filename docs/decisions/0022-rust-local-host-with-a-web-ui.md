# ADR 0022 Rust local host with a web UI

Status: Proposed

Date: 2026-10-14

Implementation: Not started

Would supersede the Node deployment assumptions in ADRs 0005, 0016, 0018,
and the service packaging portion of ADR 0021 after explicit acceptance and a
verified cutover. Their product, persistence, and trust contracts remain inputs.

## Context

The owner asked whether Rust is a better foundation for Patchfleet than the
current Node application. The next product focus combines project-scoped teams,
durable owner inputs, automatic ADR drafts, and a local memory service available
to agents. Reliable background scheduling and process ownership are concrete
host responsibilities independent of an open browser.

The existing local product uses Node/Next.js. Its main page reads server-side
projections directly, and mutations use dynamic HTTP routes. Its launcher polls
Cloud, while team advancement is triggered by observation and owner actions.
Changing language alone does not supply autonomous scheduling, isolated task
worktrees, or durable command recovery.

No measured Node performance bottleneck or compiled Rust packaging comparison
has been established. This proposal is about host ownership and distribution,
not a claim that coding-agent inference will become faster.

## Proposed direction

Make one Rust host the eventual owner of local domain validation, the canonical
event writer, projections, project memory, provider processes, bounded team
scheduling, and outbound Cloud sync. The same executable can provide lifecycle
commands, an authenticated loopback API, and a thin stdio MCP bridge. The bridge
calls the owning host rather than opening the database as another writer.

Keep the React interface and adapt its data access to the versioned host API.
During migration, Next.js can remain the temporary presentation layer. The
eventual host may serve static web assets once dynamic server dependencies are
removed. The current application cannot simply be exported unchanged: request
dependent routes and runtime projection reads must move to the host first.

Evaluate Tokio/Axum for the host API and the official Rust MCP SDK for scoped
agent tools. Evaluate the native LanceDB crate for the embedded memory index;
SQLite with sqlite-vec remains an alternative if its measured portability and
package footprint are better. These are candidates, not selected versions.
Preserve canonical acceptance facts separately from the rebuildable index.

The optional Cloud application retains its existing web stack and public
protocol. A desktop wrapper is a separate packaging choice after the headless
host works. A Rust core does not require a native UI or Tauri now.

## Migration boundary

1. First compare a bounded Rust host proof on disposable data with the existing
   event contract and proposed project-memory behavior. Include exact accepted
   decision retrieval and denial of cross-project access before vector search.
2. Establish JSON event/projection fixtures and versioned API contracts before
   moving UI routes or provider side effects. Preserve receipt meaning,
   idempotency, corruption handling, and crash-tail recovery.
3. Transfer writer ownership at a controlled cutover. Node and Rust must never
   append to the same canonical log concurrently. New memory facts must use the
   owning host's writer; a sidecar database cannot become a second authority.
4. Move provider and scheduler behavior in bounded slices with cancellation,
   restart, concurrency-limit, and worktree-isolation checks. Reuse existing
   provider contracts rather than assuming a new language adds capabilities.
5. Retire the temporary Next runtime only after UI flows use the host API and
   static asset packaging passes. Keep data rollback explicit: an older reader
   cannot be assumed compatible with newer event versions.

If Rust is accepted, build new project-memory runtime slices against this
target rather than creating a new Node memory subsystem solely to port it.
The accepted source and ADR approval policies in ADR 0021 do not change.

## Evidence required before acceptance and release

- A bounded proof compiles, runs, and replays fixtures without modifying live
  user data; source capture survives restart and decision status stays exact.
- Compare startup, idle memory, binary/assets footprint, build complexity, and
  installation requirements with the packaged Node host on the same machine.
- Validate macOS and Windows lifecycle behavior, including child-process
  cleanup and cancellation. Verify every platform advertised for release.
- Preserve host/origin validation, authenticated local mutation, project scope,
  Cloud redaction, and the existing permission limits. Loopback binding and
  Rust's type system do not establish authorization by themselves.
- Account for embedding model downloads, native libraries, platform-specific
  builds, and installed provider CLIs. A compiled host is not a guarantee that
  the whole product is one dependency-free file.

## Consequences

Rust makes an independently packaged local core possible, but introduces a new
toolchain, platform release work, and a temporary UI/API migration boundary.
Existing validated behavior is the migration reference. A full simultaneous
rewrite of the host, UI, and Cloud is outside this proposal.

The repository's current Node runtime rules remain in force until this proposal
is accepted and implementation has an approved bounded task card.

## References

- [ADR 0005](0005-single-node-web-deployable-first.md)
- [ADR 0016](0016-package-the-node-host-before-a-desktop-shell.md)
- [ADR 0018](0018-support-node-22-and-24-for-the-public-beta.md)
- [ADR 0021](0021-project-scoped-local-decision-memory.md)
- [Task 0022](../plans/0022-project-memory-and-decision-service.md)
- [LanceDB Rust crate](https://docs.rs/lancedb/latest/lancedb/)
- [Axum](https://docs.rs/axum/latest/axum/)
- [Official Rust MCP SDK](https://docs.rs/rmcp/latest/rmcp/)
- [Next.js static export boundaries](https://nextjs.org/docs/app/guides/static-exports)
