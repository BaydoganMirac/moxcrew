# Verified branded campaign assets

Updated: 2026-11-05

Status: Complete bounded deterministic I4/I5 path; generative quality remains pending

Plan: [Task0096](../plans/0096-verified-branded-campaign-assets.md)

**Project tools → Create outputs → Create branded asset pack** now stages exact
owner-approved copy, selected formats, Project colours and optional frozen
PNG/JPEG hero/logo resources in Chat. Nothing executes from the form. The owner
still sends the request, reviews the plan and starts work explicitly.

The typed `images.campaign` operation reuses the isolated local Chrome worker.
Page scripts, network access and downloads remain disabled. It accepts one to
four unique `square`, `landscape`, `portrait` and `thumbnail` formats, checks
text/background, muted/background and primary/background contrast, embeds
optional frozen visuals as data URIs and rejects unknown input, duplicate
formats, duplicate hero/logo sources, unsafe images and copy that cannot fit.
Each optional visual is capped at 512 KiB, both together at 768 KiB, and Chrome
must decode its exact bytes before any campaign output can complete.

Each requested format produces an editable SVG and exact-size PNG. The package
also includes canonical content, a source/output-hashed manifest and a human
review summary. The manifest distinguishes deterministic checks from the copy,
visual taste, trademark and campaign-performance judgments that remain human.

## Verification

- Focused worker coverage produces the full 11-file, four-format pack through a
  fixed CDP peer, recomputes every asset/supporting-file hash, pins an optional
  hero source hash and rejects duplicate formats and low-contrast palettes.
- Focused desktop coverage proves exact title/copy/palette/format/resource
  staging, zero direct Host commands and scroll-reachable action controls at
  800×560 in Turkish/English and light/dark themes.
- Google Chrome 156.0.8078.80 rendered the retained real offline pack. The
  [manifest](../screenshots/branded-assets/campaign-manifest.json) is 2,664 bytes,
  SHA-256
  `eb1a1bc328ec86abcecaa35bb1bc61257e8b46a8e958371a9c0acb2577fed67a`.
  It records contrast ratios of 14.75:1 for text/background, 5.72:1 for
  muted/background and 5.35:1 for primary/background, with network, page scripts
  and downloads all disabled.
- The retained checked PNGs are
  [square 1080×1080](../screenshots/branded-assets/campaign-square.png),
  [landscape 1200×628](../screenshots/branded-assets/campaign-landscape.png),
  [portrait 1080×1350](../screenshots/branded-assets/campaign-portrait.png) and
  [thumbnail 1280×720](../screenshots/branded-assets/campaign-thumbnail.png).
  Every IHDR dimension, byte count and manifest hash was recomputed after the
  final visual inspection; the matching SVG files remain editable.
- The real 1600×1120 Retina-equivalent Turkish dark minimum-window capture
  [campaign-form-tr-dark.png](../screenshots/branded-assets/campaign-form-tr-dark.png)
  is 300,522 bytes, SHA-256
  `af9f6209405ed8449c462301db1f574053961c5f7cb9a89068ccafbb67b582f7`.
  It shows the I4 campaign Project, exact no-model/no-publication policy and the
  start of the owner-copy form in the native app; automated geometry covers the
  remaining scrollable fields and action.
- A second real-Chrome pass decoded and embedded the native Mox PNG as a frozen
  hero before producing a checked square SVG/PNG pair. The final current-diff
  worker/readiness/form tests all pass after this decode gate was added.
- The sequential all-target/all-feature workspace suite passes with 604 tests,
  16 explicit fixture/manual ignores and zero failures. Strict workspace Clippy,
  Rust formatting, the Python syntax check and diff checks pass. The complete
  repository-required Node 24.22.1 legacy/reference suite passes 153/153.

## Limits

This closes a bounded deterministic I4 cover/card path and the multi-format
consistency portion of I5. It does not replace I1/I2 generative image quality,
apply arbitrary third-party brand manuals, decide copy, prove aesthetics or
trademark rights, measure campaign conversion, publish assets or upload them.
Optional source images are format/size/hash checked, but their crop and meaning
still need human review. Windows-native behavior, signing and public release
remain separate gates.
