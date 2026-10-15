# Automatic sourced ADR drafts

Updated: 2026-10-15

Status: Complete for approved Task0035; independently reviewed and locally packaged.
[Task](../plans/0035-automatic-sourced-drafts.md),
[ADR0036](../decisions/0036-consented-provider-draft-extraction.md),
[protocol](../extraction-protocol.md).

## Behavior

Free Local projects optionally enable source-to-ADR drafting using the installed
Codex account. The Turkish/English Memory view discloses provider upload/usage
and requires consent. Enable applies to subsequent owner source creation/edits;
existing sources have an explicit one-at-a-time queue action. No model or vector
database is bundled. Cloud subscription/publication boundaries are unchanged.

V7 stores settings, source-version job identity, attempts, outcomes and generator
provenance in the existing append-only log. One bounded inference runs outside
Local owner operations. Exact quotes and source versions are validated before
existing V4 proposals are written. Generated details identify Codex, extractor,
evidence and possible conflicts/duplicates. Only owner acceptance makes a rule
active; agent tools still read accepted ADRs only. Source edits/disable cancel
stale work. Restart/shutdown interrupts active work; failure needs manual retry.
No provider output or source text is added to default operational Cloud exports.

## Verification

Core commit8d4c953:12 extraction checks cover command idempotency, every completion
crash prefix, binding before proposal, source/policy revisions, exact evidence,
whole-batch rejection, conflicts/duplicates, manual attempt bounds, global queue
limits, maximum escaped UTF16 input, partial decision context and output canaries.
Desktop commitsf571103/4589b3:87 checks include actual TR/EN control clicks, consent,
current source/policy guards, latest10 partial-list copy, retry attempt0 and
complete generated provenance. Provider checks exercise real synthetic native
children, JSONL, tools/malformed/oversized results, deadlines, cancellation and
process-tree cleanup. Host integration exercises accepted-only agent reads,
default-off/no-backfill, generated quote preservation, edit cancellation,
no-ADR questions, responsive Local actions and restart/manual retry.

Installed Codex0.155.1 flags and strict configuration were checked in an isolated
fresh HOME through initialize only. No model turn, credential/config copy or
live provider request was made. Deterministic native responses establish the
application contract, not real model extraction quality.

## Remaining release limits

Real model quality and compliance require a separate consented provider run.
Windows/Linux native runtime and public signing/notarization remain unverified.
Cloud agent attachment is the next distinct memory integration slice. No private
Cloud code, deployed service, owner data or Applications copy was modified.

## Final checks and review

262 Rust workspace all-feature tests pass; four explicitly manual historical
checks remain ignored. All149 legacy parity/protocol tests pass. Workspace
formatting and strict all-target/all-feature Clippy pass. No new crate was
added; existing Windows bindings enable Job Objects/ToolHelp for extraction.
The full Windows core check is blocked by the existing ring build's missing
MSVC C headers. Exact production provider source and extraction tests pass
Windows cross-target strict Clippy in an isolated harness with only Store
helpers stubbed. This is type checking, not a Windows runtime proof.

Independent review closed four findings: an older failed job remains reachable
through its bounded current-source page; positive running/completed/no-ADR/
duplicate states have truthful TR/EN copy; consent explicitly includes relevant
historical accepted ADR context; successful providers cannot leave helpers.
Provider commit899b0c5 captures the Unix process group before wait and cleans it
on every outcome. Windows starts extraction suspended, binds a kill-on-close
Job Object and resumes only the verified primary thread. An independent helper
reproducer confirms the previous survivor now exits after a successful result.
Twenty provider integrations plus the private deadline check pass; six real
Host integration checks include11-job older failure recovery. No remaining
concrete code blocker was reported.

Actual native OpenGL frames were inspected in Turkish/English: default-off
[consent](../screenshots/native-memory-0.11-tr.png),
[enabled minimum window](../screenshots/native-extraction-memory-0.11-en.png),
[generated Turkish](../screenshots/native-extraction-decisions-0.11-tr.png) and
[English](../screenshots/native-extraction-decisions-0.11-en.png) drafts. The
checkbox starts empty, Enable is disabled until consent, generator/exact quote
are visible and acceptance/rejection remain explicit. These are deterministic
canonical provider fixtures, not a model-quality demonstration.

## Native0.11.0 artifact

- DMG: `dist/native/Patchfleet_0.11.0_aarch64.dmg`, 6,852,114 bytes. SHA-256: `d3d01dc2c073b1bac7512af6afe648ee1a7f0459109795522f0e416f785db0d8`.
- Packaged executable: `dist/native/Patchfleet.app/Contents/MacOS/patchfleet-desktop`, 13,409,376 bytes. SHA-256: `d81c7e597e91dfedfcd7190749b83c82cf472df0d579fdc66475584f43f433f5`.
- Release executable before packaging: `target/release/patchfleet-desktop`, 13,469,568 bytes. SHA-256: `2c46829566e9bb5f6634183714d216789cbddf3fb3166f1e31a80d65b1e6d6db`.

Feature-off App signature,0.11.0 version, bundle identifier, five folder-purpose
keys, absence of smoke controls and compressed DMG integrity pass. Framework
links contain no WebKit runtime. Explicit-path LaunchServices launch used fresh
temporary data and failing provider stubs: PID57916, window1152×822, RSS108720KiB
(106.2MiB), accumulated CPU0.13s after eight seconds. Normal close ended the
process and released writer.lock. These are small empty-state UI measurements;
provider inference and large retained histories are not included.

OS window capture remained unavailable under host privacy permissions; actual
OpenGL frames above were inspected separately. The package is macOS ARM64 with
an ad hoc development signature; public notarization and Windows/Linux runtime
checks remain outstanding. Applications was not replaced. Native0.11 is required
after V7 commands; rollback uses the stopped pre-upgrade backup.
