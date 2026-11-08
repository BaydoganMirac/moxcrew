# Adaptive failure recovery

Updated: 2026-11-08. Task0104.

Status: Local recovery control implemented and verified with a native fake peer.

## Delivered

- Adaptive missions no longer terminate on the first failed branch when an
  already prepared review member can evaluate the terminal dependency.
- The recovery reviewer starts under the frozen mission provider, ability and
  budget. It receives only original inputs and completed upstream outputs.
- V19 `project_workflow.task_allocated` facts distinguish this recovery start
  from ordinary V11 allocations without changing old replay semantics.
- V20 checkpoint facts may retire durably proven failed, blocked or interrupted
  work and append the existing bounded replacement graph.
- Retired failed tasks keep their immutable historical work identity while their
  projected status remains retired.
- Missions without a viable reviewer still fail fast; a reviewer that finishes
  without a checkpoint also leaves the original failure terminal.

## Verification

- focused native peer path: producer failure → reviewer start → evidence-backed
  replan → failed branch retired → replacement/evaluator graph appended;
- V19 allocation and V20 checkpoint version assertions;
- forced V20-to-V18 downgrade rejection;
- deterministic full-event replay after the accepted recovery checkpoint;
- direct-dependency guard proving that an unrelated parallel reviewer cannot
  hold a failed mission open;
- full Rust workspace: core `146 passed / 13 ignored`, desktop `293 passed`, all
  non-fixture integration suites green;
- legacy Node 24 parity: `153 passed / 0 failed`;
- `cargo fmt --check`, workspace Clippy with warnings denied and `git diff
  --check` green.

## Remaining gates

Fixture correctness does not establish whether a live provider notices the
right failure, selects a useful replacement or writes good evidence. Those
cross-domain live-provider and human usefulness checks remain release evidence.
