# Task0060: Atomic owner exports

Status: Complete; macOS runtime and full workspace verification pass

Updated: 2026-11-04

Authorization: “Windows için acele etme onu Windows PC'de test ederiz ama geri
kalanını çöz; Mox mükemmel olmalı.” This task closes the known Unix export
failure gap without widening the deferred Windows result-access scope.

## Outcome

Single-file and delivery-package exports publish a complete checked file or no
Moxcrew-created public destination. A failed source read, size check, write or
flush cannot expose a partial owner-facing file. A destination created by
another process remains untouched and produces the existing collision result.

## Contract

- Stage bytes in a new private sibling file opened through the pinned owner
  destination directory.
- Sync the staged bytes before one atomic, no-replace rename publishes the
  requested name. Sync the directory after publication.
- Preserve the existing absolute-path, internal-directory, no-overwrite and
  Unix-only gates. Windows remains fail closed until its native result-access
  work is tested on Windows hardware.
- Reuse the same staged destination for one-file and ZIP delivery exports.
- Keep V8/V12 requests, receipts, replay and Cloud allowlists byte-compatible;
  this task changes only the local side-effect implementation.
- Clean up only the random Moxcrew staging name. Never unlink the public target
  after another process may have replaced it.

## Verification

- Inject a source failure after an owner replacement appears; the replacement
  survives and no Moxcrew staging file remains.
- Prove the requested path is absent before commit and contains complete bytes
  after commit.
- Prove a commit-time collision preserves the other process's bytes.
- Keep real Host single-file export, delivery ZIP, exact receipt/restart,
  traversal, hash and collision tests green.
- Run workspace tests with local process/loopback permission, strict Clippy,
  formatting and diff checks.

## Boundaries

No new event version, dependency, provider call, Cloud field, UI authority,
automatic overwrite, Windows support claim, signing or publication.

## Completion

Implemented in the shared single-file and delivery-package destination path.
The two focused race/failure tests pass, including a partial staged write before
failure, an owner replacement appearing during the failed copy and a collision
at publication. The complete workspace/all-target suite passes **524 tests with
zero failures and six opt-in tests ignored**. Strict workspace/all-target/
all-feature Clippy (`-D warnings`), workspace formatting and `git diff --check`
pass. The current host is macOS; Windows remains deferred as authorized, and a
native Linux runtime rerun remains a release/CI gate rather than claimed local
evidence.
