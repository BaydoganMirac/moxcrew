# Verified static landing site

Updated: 2026-11-05

Status: Complete bounded path; deployment and content-performance review pending

Plan: [Task0092](../plans/0092-verified-static-landing-site.md)

**Project tools → Create outputs → Website / landing page** now stages exact
owner-approved content in Chat without starting production or publishing
anything. The Turkish/English form accepts a title, eyebrow, headline,
supporting copy, two to six feature rows and one local-anchor or HTTPS call to
action. It rejects incomplete rows and unsafe links before creating a typed
`web.site` request.

The bundled standard-library worker escapes every owner string and emits an
editable `index.html`, canonical `content.json`, `site-manifest.json` and
`summary.md`. The page contains no script, tracker, remote font or remote asset;
its CSP denies network/resource loading, and the output includes semantic
landmarks, a skip link, visible keyboard focus, reduced-motion handling and a
responsive single-column layout. Every generated file is reopened byte for byte
and its size/hash is pinned before the result is accepted.

## Verification

- Deterministic core coverage rejects `javascript:` links and unexpected input,
  proves hostile closing tags/scripts remain escaped, validates the network-free
  CSP, reopens the output and recomputes every manifest hash. Repeating the same
  request produces identical bytes. Desktop coverage proves exact content
  staging, no direct Host action or publication, and action reachability at
  800×560 in Turkish/English and both themes.
- The retained [index.html](../screenshots/static-site/index.html) is 7,211 bytes
  with SHA-256
  `183a7fe37c69790a0093bbf13220a61c9ce26b9dfbcb9a7e92ad5d76db854004`.
  [content.json](../screenshots/static-site/content.json) is 725 bytes with
  SHA-256
  `d15ccf6d91efab36e6cc7ff90a7120fa03fab4e3f3dee92f91c545698a8bcd6d`;
  [summary.md](../screenshots/static-site/summary.md) is 621 bytes with SHA-256
  `4b8d93073784f2e5510a74744405ba2b6dc2a33b28d9845efa29df84ffc5483b`.
  The retained [site-manifest.json](../screenshots/static-site/site-manifest.json)
  is 817 bytes with SHA-256
  `49180648b05263ee01ffc68657ad072cce42615516d35122d37ae42db5955683`.
- Google Chrome 156.0.8078.80 rendered the real local file at 1440×1100 and
  500×1400 without external requests. The
  [wide capture](../screenshots/static-site/site-wide-1440.png) is 161,999 bytes
  with SHA-256
  `ba8b0bac782b7a2d816cc3a52529649a0c4c99b9cd22e382896ad161671aa878`;
  the [narrow capture](../screenshots/static-site/site-narrow-500.png) is 120,331
  bytes with SHA-256
  `ba8f1ff9daf61f3923fdb456bc5c88ab1bfe3a3895cce989fe0a944bd02415a9`.
  The narrow render switches to one column, hides the secondary nav link and
  keeps the CTA, feature text and decorative mascot within the page.
- The real 2560×1760 Turkish dark native capture
  [site-studio-tr-dark.png](../screenshots/static-site/site-studio-tr-dark.png)
  is 615,779 bytes with SHA-256
  `5aee4f1edf796617c169c649418903076f27a79c1b222a196322fa551c03c93f`.
  It shows the W1 project, exact no-publication policy, language choice and all
  owner content/CTA fields in the native desktop app.
- The retained run used Python 3.9.6 and Google Chrome 156.0.8078.80. Focused
  worker, desktop, retained-output and native-smoke checks pass. The sequential
  all-target/all-feature workspace run passed 596 tests with 15 explicit
  real-tool, PostgreSQL, evidence-writer or manual timing ignores and no
  failures. Strict all-target/all-feature Clippy with warnings denied,
  formatting, Python syntax and diff checks pass.

## Limits

The page is a bounded visual system, not arbitrary HTML/CSS/JavaScript. No
domain, deployment, publication, analytics, form submission, remote asset or
account action occurred. Structural accessibility and representative responsive
renders are verified, but screen-reader/manual keyboard review, browser-matrix
coverage, copy effectiveness, conversion performance and SEO outcomes still
need human and deployment-specific review. Windows-native behavior, signing and
public release remain separate gates.
