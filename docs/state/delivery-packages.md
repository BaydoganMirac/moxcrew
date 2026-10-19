# Project delivery packages

Updated: 2026-10-19

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0047](../plans/0047-project-delivery-packages.md).
Decision: [ADR0046](../decisions/0046-project-delivery-archives.md).
Host details: [core contract](delivery-packages-core.md).

## Owner flow

Project Outputs keeps the accepted native design and existing previews, revisions
and single-file export. A visible name/path search and file-type picker filter
the actual output list. **Create package** opens deliberate multi-file selection.
Select shown, clear, per-file removal and an inspectable selection summary retain
files hidden by a filter. Count/size stay beside a primary **Export package**
action. Changed or unavailable selections stay visible for correction and block
export. There is no implicit replacement with a newer version.

Export freezes the Project/work/path/SHA-256 identities before the native ZIP
save dialog. Cancel emits no command. A pending request survives a simultaneous
automatic read and later view/Project changes without retargeting it. Replacing
the Host invalidates it. Host validation rechecks actual source bytes before any
destination is created. Existing files are never overwritten.

The standard UTF-8 ZIP keeps full sanitized basenames/extensions in numbered
folders (`files/01/name.ext`) and includes `manifest.json` schema1 with Project,
work, source path, archive path, SHA-256 and byte count. No prompts, credentials
or absolute source paths enter the manifest. Original source files remain intact.

## Limits

Require 1–32 files from one Project, at most 25 MiB per file and 64 MiB total
source bytes. ZIP uses STORE without compression/ZIP64; packaging needs no
Python, FFmpeg, dependency or provider process. Windows export remains explicitly
unsupported pending pinned no-reparse file operations. An I/O failure can leave
an incomplete destination and reports failure; it never deletes a public path
another process might have replaced. Existing V12 audit/replay behavior is reused;
no event schema, Cloud contract or external account action was added.

Archive names preserve sanitized Unix basenames. Windows-specific reserved names
such as `CON.txt` can require renaming when a recipient extracts the archive on
Windows; cross-platform extraction compatibility is not claimed.

## Verification

- Full workspace/all-feature run before final review corrections: **423 passed,
  0 failed, 6 opt-in fixtures ignored**. After the narrow corrections, the affected
  complete desktop suite passed **164 tests**, and all **6 focused Host delivery
  tests** passed. Unaffected provider fixture suites were not repeated.
- Selection/filter tests use actual pointer typing and repeated keyboard choices.
  They cover hidden selections, stale/missing files, package limits, busy reads,
  dialog cancellation, frozen scope and Host replacement. Existing single-file,
  revision, setup and approval tests remain green.
- Strict workspace/all-target/all-feature Clippy and workspace formatting pass.
- A real Host ZIP reopens in Python's independent standard `zipfile` reader:
  CRC32, UTF-8 flags, every manifest length/hash, colliding Unicode basenames and
  a 253-byte `.pptx` name pass. Three sources total 61 bytes; archive 2,405 bytes,
  SHA-256 `43502a4868341418e0b719bddd28e7a01ddb04e9f3c93f7994818c029df7edc6`.
  This fixture verifies archiving, not Office generation. Local evidence:
  `branding/moxcrew/delivery-packages-qa/real-output/qa.json` in the sibling output
  directory, with `delivery-example.zip`.
- **12 native captures**: delivery selection in TR/EN, both themes, minimum 800x560
  and normal windows; normal Files in TR/EN and both themes at minimum size.
  Independent source/output review found no remaining blocker. Local evidence:
  `branding/moxcrew/delivery-packages-qa/native/qa.json`. Fixture executable SHA-256:
  `125ba6f7c13c367de8849ff4a5b3d05f26577657611e180fe038921566c2d88a`.
- Final source: `41512ee`, following `c7d18de`, `5306e50`, `c8c49e0` and `ca828e7`.

## Development package

`dist/native/delivery-packages/Moxcrew_0.12.0_delivery-packages_aarch64.dmg`

Version remains 0.12.0. This macOS ARM64 development package is ad-hoc signed,
not publicly notarized. License resources, strict signature verification,
smoke-feature exclusion, normal native launch/close, writer-lock release and
DMG integrity verification pass. Evidence: `dist/native/delivery-packages/qa.json`.

- Executable: 16,571,616 bytes; SHA-256
  `257eb9c17a8aed866f2609f5426618025a9905a47efb55995feb1b48f19b7300`.
- DMG: 8,539,820 bytes; SHA-256
  `380806c7cf4dad4d2e6bb8c9499f1dbe81c6535424958fca7154b6be52e7d212`.

Native QA uses isolated disposable data without inference, provider probes or
external account writes. Windows/Linux execution, real provider inference,
live accounts and deliberate disk-full injection were not checked. Public
release signing/notarization, push and deployment are outside this local change.
