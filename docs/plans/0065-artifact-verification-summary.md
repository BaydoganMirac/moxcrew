# Task0065: Artifact verification summary

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the non-Windows product and make Mox
excellent. Windows remains deferred to a Windows PC. This slice closes the user
experience gap between an exact Output Studio preview and an understandable
statement of what that preview actually proves.

## Problem

Output Studio already reads a hash-pinned artifact and can reopen text, Office
structure, images or media metadata. The UI shows the preview but does not state
which evidence was verified or which quality checks still require human review.
Users can therefore mistake “a preview exists” for layout, playback or content
quality proof.

## Scope

1. Derive a small verification summary only from the existing exact
   `project_artifact_read` response.
2. Always distinguish exact version/hash verification from format inspection.
3. Explain the strongest proven format evidence for readable text, reopened
   Office structure, loaded image bytes and parsed media metadata.
4. Keep unsupported formats honest and name the remaining human review for
   content accuracy, layout, visual fidelity or playback.
5. Localize the summary and cover exact scope/hash isolation plus minimum-window
   rendering.

## Non-goals

- No invented quality score, semantic correctness claim or model evaluation.
- No new durable event, artifact schema, background verifier or tool execution.
- No automatic opening, export, revision or external upload.
- No Windows result-access claim.

## Acceptance

- Verification appears only for the exact selected Project/work/path/hash.
- Exact bytes and format evidence are separate; unsupported inspection never
  becomes a green format claim.
- Every supported preview kind has a concise proven fact and explicit review
  limit in Turkish and English.
- Focused and full workspace tests, strict Clippy, formatting and diff checks
  pass with native visual evidence.

## Verification

- Focused desktop verification: 2 passed.
- Full desktop package: 242 passed.
- Full serial workspace: 536 passed; 6 opt-in external-environment tests
  intentionally ignored.
- Strict workspace Clippy, formatting and diff checks passed.
- Native Turkish dark-theme Output Studio capture reviewed at 2560×1760.
