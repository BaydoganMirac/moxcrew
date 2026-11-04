# GitHub product surface

Updated: 2026-11-04

Status: Implemented, locally verified and published to `origin/main`.

## Outcome

The public repository now explains Moxcrew before its implementation history.
The root README leads with one outcome—turn one goal into an inspectable local AI
crew—then shows the real native product, the owner-controlled workflow,
local-first differentiation and an honest development-preview call to action.
Architecture, protocol and release evidence remain linked rather than occupying
the acquisition path.

Three English/light 800×560 native captures show the current welcome, Decisions
and agent catalog. The README names the absence of public downloads and does not
claim signed distribution, complete live-provider quality or moderated usability.

Package, Cargo and packager descriptions now match the general-purpose product
instead of the historical coding-only command-center wording. GitHub About uses
the same value proposition and discovery topics for AI agents, orchestration,
local-first desktop, Rust/egui, Codex, Claude Code, Gemini CLI and productivity.
The repository homepage is deliberately unset because `moxcrew.com` still serves
a Plesk default page.

## Boundaries

Compatibility package/binary identifiers remain `patchfleet` where changing
them would break installation or data continuity. No product behavior, protocol,
Cloud boundary, website, release, package publication or discussion setting
changed. No credential or private Cloud implementation was added.

## Verification

- `cargo test --locked --workspace --all-features -- --test-threads=1`: 524
  passed, 6 explicitly external/manual fixtures ignored.
- The process-tree timing test passed three isolated repetitions and the complete
  workspace after its startup budget changed from 250 ms to 500 ms; the 800 ms
  descendant-escape assertion remains intact.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  passed.
- `cargo fmt --all -- --check`, Cargo metadata, npm package metadata, README
  referenced-file checks and `git diff --check`: passed.
- GitHub About description and ten repository topics were read back through the
  GitHub API after update.

The product/UI/README delivery was published in commit `7779a53` and verified as
the remote `main` head before this final state handoff was recorded.
