# Task 0040: Project Create studio

Status: Complete for bounded foundation; reviewed and locally verified

Updated: 2026-10-19

Owner authorization: explicitly start feature implementation and accompanying
interface work under the accepted 1.0.0 plan. This is the first bounded delivery
  within that broad release; the eight-area/forty-ability target is unchanged.

## Outcome

A native Create screen lets an owner choose a Project and an inspectable ability
recipe, write a goal, queue a real project-scoped job without a Git folder,
explicitly start/cancel it through supported Codex control, and inspect/export
actual output files. Recipes guide provider work; listing forty recipes does not
prove that forty file formats or external tools are installed or verified.

Owner feedback after the first native UI: the screen feels complex and difficult
to use. The authorized interface refinement makes the goal prompt dominant,
keeps Project and selected recipe clear, and moves the full catalog/advanced
fields into deliberate disclosures. Preserve every scoped draft, provider gate
and explicit start. Queueing remains a preparation step, never automatic
execution. Primary navigation should emphasize Create, Projects and Tasks;
secondary tools remain reachable without competing with the first task.

## Contract

- A compiled catalog uses the forty IDs from Task0039 and eight categories.
  Each record has id/category/name/description/starter/instruction. Name,
  description and starter have en/tr variants. Instructions are inspectable
  guidance and grant no tool access.
- `project_work_enqueue` accepts projectId, expectedProjectRevision, abilityId,
  title and instruction plus the existing command envelope. It validates current
  project scope and creates a private per-job directory under the local data
  directory. Reuse durable work requests/receipts and explicit project binding.
- General jobs use only their exact canonical managed directory. Existing
  ordinary worktree registration and Git validation keep their current meaning.
  Both Codex prepare and start validate the managed path through a separate
  trusted local route; a user payload cannot opt arbitrary folders into it.
- Derive `projectJobs` from current work/project binding and managed directory
  identity. Snapshot `abilities` is recipe metadata. Never infer job ownership
  from a display name or a shared folder.
- `project_results_read` accepts projectId/workItemId and returns those IDs,
  workStatus, bounded items `{path,name,bytes,textPreview}` and truncation status.
- `project_result_read` additionally accepts a relative path and returns
  projectId/workItemId/path/text/bytes for bounded supported text files.
- `project_result_export` accepts projectId/workItemId/path/targetPath, copies
  only a validated result to an owner-selected absolute destination and never
  overwrites an existing file. Traversal, symlink escape, cross-project access
  and access to canonical local files must be rejected.
- Queued, running, partial, failed and complete are distinct. Do not treat an
  enqueue receipt or an arbitrary provider-created file as verified ability
  completion. Match async results/receipts to exact project/job/command scope.

## Ownership

- Core builder: `crates/moxcrew-core/src/creation.rs`, managed-path prepare/
  start variants in `providers.rs`, `crates/moxcrew-core/tests/creation.rs`
  and necessary provider fixtures only.
- UI builder: `crates/moxcrew-desktop/src/ui.rs`, `i18n.rs`, new `create.rs`
  child module; preserve other drafts and existing navigation behavior.
- Coordinator: catalog assets/module, `host.rs`, `work.rs`, `lib.rs`, native
  `main.rs`/`bridge.rs` integration, shared contracts, plan/ADR/state, integration.
- F0 feasibility worker: disposable presentation/video tool experiments only;
  no product files or dependency installation.
- Reviewer: inspect stable integrated artifact and checks before final handoff.

## Verification

Use real temporary directories and fake supported provider sessions for managed
prepare/start/cancel and retry boundaries. Cover stale revision, duplicate
commands, foreign project/job, ordinary Git-path regression, path traversal,
symlinked roots/results, missing/oversize/unsupported files, bounded listing,
export collision and preserved inputs. Confirm restart retains job identity.

Native checks cover actionable empty/provider-error/busy states, exact scope
matching, draft preservation, result rendering and export-dialog cancellation.
Exercise TR/EN, light/dark and minimum window layout. Run affected core/desktop
tests, fmt/Clippy, release build and actual native rendering evidence. F0 tool
proof is development feasibility, not a shipped renderer or a live AI proof.

## Excluded from this slice

No new API credentials/provider purchase, dependency download, Node/WebView,
new execution authority, Cloud payload/protocol/billing change, global skill
installation, arbitrary remote shell, automatic execution, public publication
or deployment. Live provider/output quality and the remaining 1.0.0 paths stay
explicitly unverified until independently demonstrated.

## Handoff

Local implementation commits, independent review, checks/native evidence and
updated Create studio and 1.0.0 state. Do not mark 1.0.0 or all forty abilities
complete on the strength of this foundation.
