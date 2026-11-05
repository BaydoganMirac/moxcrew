# Owner-visible presentation editing

Updated: 2026-11-05

Status: Complete; locally verified

Plan: [Task0094](../plans/0094-owner-visible-presentation-editing.md)

**Project tools → Edit presentation slide** now exposes the existing verified
`presentations.revise` worker in the native desktop app. The owner selects one
exact Project PPTX and one one-based slide number, then supplies a bounded cover,
statement or content replacement, optional speaker notes and an optional real
render check.

The form only stages a typed request in Chat. It does not execute a Host command,
call a provider/model or mutate the presentation. The resulting instruction
keeps plan review and Start explicit and requires the worker to report its
exact-source, exact-target and byte-identical non-target-slide checks.

The execution contract remains documented in
[branded visual presentations](branded-visual-presentations.md). Its focused core
coverage reopens the revised PPTX, verifies the selected slide and proves every
non-target slide XML part stayed byte-identical.

## Verification

- Focused desktop tests prove exact resource/slide/layout staging, owner text,
  render intent, zero direct Host commands and primary-action reachability at
  800×560 in Turkish/English and light/dark themes.
- Existing focused core coverage proves the actual revision and exact
  preservation contract.
- The sequential workspace/all-target/all-feature suite passes with 601 tests,
  16 explicit external-fixture/manual ignores and zero failures. Strict
  workspace/all-target/all-feature Clippy, formatting and diff checks pass.
- A real Turkish dark native capture is retained as
  [presentation-revision-studio-tr-dark.png](../screenshots/presentation-revision/presentation-revision-studio-tr-dark.png),
  574,486 bytes, SHA-256
  `a9efa6bc79e12e98d196c61d013c77ea4fcf78f0cb7c9a9cd0008c046a4805a8`.
- The capture uses a disposable imported PPTX and invokes neither the revision
  worker nor a live provider/model.

## Limits

The form intentionally edits one slide per reviewed request. It does not reorder
or delete slides, expose arbitrary PowerPoint XML, rewrite animations/SmartArt or
claim Microsoft PowerPoint/Windows validation. Windows testing remains a
separate owner-approved gate.
