# Moxcrew native interface

Status: Complete; independently reviewed and locally verified

Updated: 2026-10-19

## Authority and identity

The owner purchased moxcrew.com and explicitly requested the Moxcrew kit and
Cloud/desktop implementation. This supersedes the earlier Dobbly name; Mox is
the selected concept-4 mascot. The owner reconfirmed the architecture as
Mox + crew: Mox names the character; crew expresses the cooperating team and
community spirit; Moxcrew names the product. Existing plan/state/ADR filenames remain stable
so prior links continue to resolve.

[Task0037](../plans/0037-dobbly-brand-interface.md) and accepted
[ADR0037](../decisions/0037-dobbly-display-identity.md) govern this slice.

## Implemented

- Portable versioned source in `design/moxcrew`: outlined Manrope Moxcrew
  wordmarks, color/mono/small icons, original Mox PNG, exact ImageGen provenance,
  licensed local web font and semantic JSON/CSS/Rust tokens. Original generation
  prompts retain their historical name; the artwork is reused unchanged.
- Native semantic light/dark styles installed once, following OS preference with
  light fallback. Font installation is unchanged and local. No new production
  dependency, persisted preference field, network asset or idle animation.
- Cobalt/ink/paper surfaces across all eight tabs and exceptional/account states;
  Tasks/Görevler display label retains the Work commands and enum. Shared fields,
  focus, status, cards, nav and notices use the same themes. Success, waiting,
  failure and running retain explicit labels. Primary actions are44px; compact
  secondary controls36px. Decorative borders and3:1 control borders are separate.
- Mox welcomes first use, including800x560, with the first-project action visible
  before scrolling in both languages and themes. The768px embedded mascot PNG
  is276,055 bytes and decodes/uploads once. Original full-resolution source is
  preserved; no render-thread file I/O or image framework was added.
- Visible window/package/permission copy and app/platform icons now use Moxcrew.
  `com.patchfleet.desktop`, executable/crate/package names, data directories,
  environment keys, API/event/command identifiers and language schema are stable.
  Installer CI uses Moxcrew.app; Apache and Manrope OFL travel inside packages.

## Verification

- Baseline263 all-feature Rust checks passed,4 existing opt-in checks ignored.
  Full post-change suite265 passed,4 ignored, including90 desktop checks.
  After the final control-border contrast correction,90 desktop checks passed
  again. All-target/all-feature strict Clippy, formatting and feature-off release
  build pass. Test-only scene/theme controls are absent from production bytes.
- Existing149 legacy protocol/package checks passed in an isolated run. An
  earlier concurrent run had3 timing-related adapter failures; no adapter logic
  or timeout was changed for this presentation task.
- Existing draft/scope, consent, revision, receipts and provider gates remain.
  All eight tabs render without commands in both styles; new checks exercise
  system theme changes/light fallback and first-project action visibility.
- Ten actual native GL scenes: TR/EN, light/dark,1280x880/800x560, first use,
  populated projects/tasks/teams/agents/providers/memory/decisions and optional
  account. Headless tests render at1x; macOS GL and OS captures are2x.
- Real feature-off App launched through NSWorkspace with empty data and failing
  provider stubs. Own bundle/PID/layer-zero window matched; normal close released
  each writer lock. Ad-hoc signature, identifier/version, all five folder-purpose
  keys and bundled licenses pass. No installed application or owner data used.
- Shared16 normal-text pairs meet4.5:1; sixteen additional control/focus pairs meet
  3:1. `design/moxcrew/tokens/contrast-checks.json` records actual ratios.

## Artifact and measurement

`dist/native/Moxcrew_0.12.0_aarch64.dmg`:7,294,667 bytes; checksum verification passes.
SHA-256 `3e6006945df21c576c259e0ce31ef12ba5e62956b54c540fcc52035a37cacb30`.
Signed App executable SHA-256
`15e7c628e0ffafeaeac987fcbe3934ddf94a7cca696239b0693676df374c5d7b`.

Same-method development measurements with empty data/failing provider stubs:

| Measure | Before | Moxcrew |
|---|---:|---:|
| Release executable bytes |13,618,368|13,982,080 (+2.67%)|
| Locally assembled DMG bytes |6,977,041|7,294,667 (+4.55%)|
| LaunchServices callback |62ms|415ms|
| Eight-second process RSS |112,048KiB|115,904KiB|
| Eight-second cumulative CPU |0.25s|0.27s|

These are single startup samples, not a cold-start or active-provider benchmark.
A visible OS window was captured at8seconds. Callback latency is not time to
first painted content. Some other disposable GL captures overlapped the sample;
no performance improvement or production workload guarantee is inferred.

## Evidence and limits

- [Packaged macOS window](../screenshots/moxcrew/production-macos-tr.png)
- [Light projects](../screenshots/moxcrew/studio-projects-en-light.png),
  [dark projects](../screenshots/moxcrew/studio-projects-tr-dark.png)
- [First use at minimum size](../screenshots/moxcrew/first-use-tr-light-min.png),
  [tasks](../screenshots/moxcrew/studio-work-en-dark.png),
  [providers](../screenshots/moxcrew/studio-providers-en-light.png)
- [Before projects](../screenshots/moxcrew/before-projects-en.png),
  [before first use](../screenshots/moxcrew/before-first-use-tr.png)

Windows/Linux native runtime, public signing/notarization, live provider/auth/
billing services, exhaustive keyboard traversal and live Cloud deployment remain
outside this local presentation verification. No DNS, migration, release,
installed-app replacement or inference call occurred. Cloud verification is
recorded in its separate repository; this file does not publish private details.

## Independent review corrections

The review checked actual control borders on canvas, surface, raised and
selected surfaces. Shared controlBorder is now light `#7A889F` / dark
`#7588A8`, matching Cloud; all32 text/control/focus pairs pass. The final
90 desktop checks, strict Clippy and native captures were repeated. Windows
NSIS/WiX upgrade discovery was corrected in `9332ecc` and independently reviewed.
The pinned0.11.8 templates preserve prior registry/install/shortcut identities
with Moxcrew display labels. Four focused identity checks pass; both templates
render with actual Handlebars and rendered WiX XML parses. The MIT notice ships
with the development package. Actual Windows compiler/install/upgrade/uninstall
remains a release gate, without any canonical data cleanup.

Independent review covered native `bb70bae`, contrast `f1e7d1c` and installer
`9332ecc`; both findings are resolved with no remaining actionable findings.
The reviewer independently repeated90 native checks,4 installer checks and all32
contrast ratios. It did not claim Windows runtime or hosted release verification.

## Repository-name follow-up

On2026-10-19 the owner explicitly requested the GitHub repository names to match
Moxcrew. The existing public repository was renamed to `BaydoganMirac/moxcrew`;
its GitHub identity, public visibility and main branch were verified unchanged.
The local origin uses the new URL. README logo/theme variants, canonical package
homepage/repository/security URLs, agent entry point and active product/
architecture/roadmap/docs headings now use Moxcrew. Installation/data/package/
protocol identifiers remain compatible.

Nine relevant CLI/installer metadata checks pass; README image paths and absence
of obsolete canonical GitHub links are checked. No Rust runtime change or new
package/release is claimed by this documentation follow-up. These local commits
have not been pushed; the pre-existing accumulated history requires an explicit
owner publishing direction under AGENTS.md.
