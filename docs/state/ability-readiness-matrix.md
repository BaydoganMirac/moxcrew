# Ability readiness matrix

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0062](../plans/0062-ability-readiness-matrix.md)

The current release catalog has forty localized recipes. Before Task0062 their
native catalog showed only a generic warning that results depend on tools and
access. Actual Python, media, Office, transcription, Chrome and image-engine
readiness existed in Project Studio but was not connected to recipe selection.

Task0062 adds a non-durable readiness view over those existing facts. Every
catalog entry now carries an exhaustive profile of required local tools,
optional enhancement tools and separately checked access. The profile uses only
the existing Python, FFmpeg/FFprobe, Poppler, Tesseract, whisper.cpp/model,
LibreOffice and Chrome identifiers plus image-engine, app-connection,
source/browsing, voice/model and awake-Host requirements.

The native catalog requests the existing bounded `project_studio_read` once when
needed. It combines the profile with the exact selected Project's tool,
image-engine and app-connection response, then shows one of these deliberately
limited claims:

- **Local checks ready:** declared required local checks passed; optional tools
  may still be absent.
- **Setup needed / Check setup:** a declared local prerequisite is missing or
  has not been read for the selected Project.
- **Access check:** source, browsing, voice/model or awake-Host access must still
  be confirmed when the work starts.

Optional tools never block a recipe. The catalog remains a selector: opening it,
checking readiness or choosing a recipe never prepares or starts work. A setup
action opens the existing Output Studio Tools, Project Create outputs/image
engine or Output Studio Apps view. Nothing is installed automatically.

## Verification

- The catalog contract covers all 40 stable IDs and rejects unknown tool/access
  identifiers.
- Desktop behavior covers loaded, missing, optional, pending and no-Project
  states; one-shot Studio reads; exact Tools/Image/Apps routing; keyboard recipe
  selection; and TR/EN dark/light geometry at the 800×560 logical minimum.
- `cargo test -p patchfleet-desktop` passed 231/231 with loopback permission.
- `cargo test --workspace --all-features -- --test-threads=1` passed **527 tests,
  zero failures, six ignored**. Two parallel full-suite attempts exposed existing
  timing-sensitive NVM/process-reaping fixtures; each passed alone and the entire
  serial suite passed. No assertion was disabled.
- Strict workspace/all-target/all-feature Clippy, workspace formatting and
  `git diff --check` pass.
- Native test-only OpenGL captures are retained at
  [catalog-tr-dark-min.png](../screenshots/ability-readiness/catalog-tr-dark-min.png)
  and
  [catalog-setup-tr-dark-min.png](../screenshots/ability-readiness/catalog-setup-tr-dark-min.png).
  Both are 1600×1120 physical pixels for the 800×560 logical viewport. The setup
  capture shows a real local check with missing whisper.cpp/model and the visible
  setup route; it does not establish speech quality or a live model.

No canonical event, Cloud field, dependency, provider execution, external
action or Windows behavior changed. Readiness still does not certify recipe
quality, live provider entitlement or successful output. The complete 40-ability
quality/evidence matrix remains release work.
