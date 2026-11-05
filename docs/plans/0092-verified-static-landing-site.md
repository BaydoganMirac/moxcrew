# Task0092: Verified static landing site

Status: Complete bounded path; deployment and content-performance review pending

Updated: 2026-11-05

Authorization: The owner asked to close the remaining non-Windows product gaps
and make Mox excellent. This card implements the existing W1 website ability
without a live provider/model call, account action, dependency installation,
deployment or publication.

## Problem

The W1 catalog entry promises working local website files, but the bundled
production worker only exposes generic interactive tables, charts and budgets.
There is no typed website operation, owner-visible site form, content manifest
or retained responsive/browser evidence.

## Scope

1. Add a bounded `web.site` operation for one static landing page with exact
   title, eyebrow, headline, supporting copy, feature rows and call to action.
2. Emit editable `index.html`, canonical `content.json` and a hash-pinned
   `site-manifest.json` without scripts, remote assets, trackers or network
   requests.
3. Escape owner content, restrict links to a local anchor or checked HTTPS URL,
   and enforce semantic, keyboard-focus and responsive layout invariants.
4. Add a native Create outputs form that stages the exact request without
   starting production or publishing anything.
5. Reopen generated files and retain narrow/wide browser previews plus native
   Turkish dark evidence.

## Non-goals

- No domain purchase, hosting, deployment, account action or publication.
- No arbitrary owner HTML, CSS or JavaScript and no external fonts or images.
- No live model, browser account or provider call.
- No claim about conversion performance, copy quality or Windows behavior.

## Acceptance

- Invalid content, unsafe links and unexpected fields fail without a result.
- Tests prove escaped content, a script-free/network-free document, exact hash
  manifest, byte-for-byte reopen and deterministic output.
- Desktop tests prove exact request staging, no direct Host action and form
  reachability at 800×560 in Turkish and English and both themes.
- Retained evidence includes real wide/narrow browser renders, native form
  capture, hashes and explicit human-review limits.
- Full tests, strict Clippy, formatting, Python syntax and diff checks pass.

## Outcome

The typed local worker, owner-visible native form, script-free editable site
package, hash manifest, exact reopen checks and real wide/narrow Chrome plus
native captures are complete. The full workspace passed 596 tests with 15
explicit ignores and no failures; strict Clippy, formatting, Python syntax and
diff checks also pass. Deployment, conversion performance, copy effectiveness,
assistive-technology review and Windows behavior remain explicit future gates.
See [verified static landing site](../state/verified-static-landing-site.md).
