# Start readiness preflight

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0063](../plans/0063-start-readiness-preflight.md)

Task0062 made local prerequisites visible in the ability catalog. Task0063
connects that same bounded evidence to the final native start actions for
Project jobs and crews.

The local `projectJobs` snapshot now includes only the stable catalog ability ID
derived from already-authorized direct work, retries, crew allocations and
Studio revision work. This is runtime summary metadata, not a new canonical
event or Cloud field. Owner instructions and other private request content are
not copied into that summary.

Chat task cards, Create output/task cards, prepared Crew missions and the general
Tasks queue now share one readiness preflight. The desktop requests the existing
bounded `project_studio_read` response when a declared local check is needed.
An unknown or missing required tool, image engine or supported app connection
keeps the native Start action disabled and exposes **Review setup**, routed to
the existing Tools, Create outputs or Apps surface. A crew combines the unique
requirements of its primary ability and all member tasks.

Optional tools never block. Source/browser access, voice/model access and the
awake-Host requirement remain visible access checks and do not become invented
local proof. Passing preflight means only that the declared local prerequisites
are present; it does not prove provider entitlement, model quality or a useful
output.

## Verification

- Direct managed work exposes `D1`; real Crew allocations expose each task's
  exact ability; Studio revision and its retry retain `W1`. The summary contains
  no instruction field.
- Native behavior tests cover pending, missing, ready and access-only states,
  unioned Crew requirements, disabled Start actions and exact setup routing on
  Chat, Create, Crew and Tasks.
- `cargo test -p patchfleet-desktop --all-features` passed **237/237** with local
  loopback fixture permission.
- `cargo test --workspace --all-features -- --test-threads=1` passed **531 tests,
  zero failures, six explicit opt-in tests ignored**.
- Strict workspace/all-target/all-feature Clippy, workspace formatting and
  `git diff --check` pass.

This is a native desktop guard, not a new authorization boundary in the Host
command protocol. Direct CLI/API callers retain the existing explicit start
contract. No tool was installed, no external action or provider turn was run
outside disposable test peers, and no Windows behavior changed.
