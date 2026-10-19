# Task0047: Project delivery packages

Status: Complete; independently reviewed and macOS development package verified

Updated: 2026-10-19

Authorization: after accepting Task0046's direction the owner requested continued
feature implementation. This bounded slice advances the approved composed-output
release scope: find actual outputs, select several and export a usable package.

## Observable result

- Output Files has search/type filtering and a deliberate package-selection mode.
- A visible selection summary shows count and bytes; Preview/revise/single-file
  export keep working. Hidden filtered selections are never silently exported:
  the selection is inspectable and can be cleared individually or together.
- Explicit Export package opens the native save dialog and freezes the selected
  Project/work/path/SHA-256 identities. Cancellation emits no command. Subsequent
  UI/project changes do not retarget the frozen request.
- The Host validates every selected file before creating the destination. Changed
  hashes, foreign/missing jobs, traversal, duplicate identities, symlinks, limits,
  internal/existing destinations and unsupported platforms fail closed.
- Export one standard UTF-8 ZIP with the original bytes under collision-free
  relative names, plus manifest.json containing Project/work/path/hash/byte
  provenance. No prompts, credentials or absolute source paths enter the package.
- A real ZIP reader must reopen the result and reconcile every hash. Failure is
  visible; no partial package is reported successful. Originals remain intact.

## Contract and limits

Native-only owner action `project_delivery_export` has Project ID, files array
and targetPath. Each file is exactly {workItemId,path,expectedSha256}. Command ID,
creation/expiry and durable requested/receipted outcomes use the existing V12
owner command infrastructure. Existing schemas/commands retain their meanings;
Cloud serializers and remote intent allowlists do not change.

Require1–32unique files, <=25MiB per file and <=64MiB aggregate. No ZIP64 or
compression is needed for already-compressed Office/media files. Reuse pinned
descriptor reads and checked no-overwrite export creation. A write failure may
leave an incomplete destination, as in existing single-file export; never remove
a public path that could have been replaced by another process. No new dependency
or Python/tool requirement for packaging.

## Ownership and verification

Core builder owns creation.rs (shared checked export helper only),
project_studio.rs (action registration/dispatch), a new delivery module beneath
it, focused core tests and delivery state. Coordinator owns all desktop code,
shared contracts/docs, integration, native captures and packaging. Reviewer is
read-only after stable commits and actual output exist.

Verify exact scopes/hash failures before destination creation, duplicate command
replay, no overwrite/internal target/symlink behavior, Unicode/colliding names,
limits, ZIP extraction/hash manifest and UI selection/filter/picker preservation
in TR/EN, both themes and minimum/normal windows. Preserve Task0046 design.
Run relevant core/desktop checks, strict workspace Clippy/formatting and build a
separate smoke-free macOS development package. No inference, real account writes,
Cloud change, push or public release.

## Completion

Implemented and independently reviewed at `41512ee`, after core `c7d18de`,
desktop `5306e50` and core `c8c49e0`. The full workspace run passed 423 tests;
the final affected desktop suite passed 164 tests and all 6 focused Host delivery
tests passed after review corrections. Strict workspace Clippy/formatting pass.
Twelve native captures, independent standard ZIP-reader checks and a smoke-free
macOS ARM64 development package pass. See [delivery state](../state/delivery-packages.md)
for hashes, evidence, exact limits and unverified platforms/accounts.
