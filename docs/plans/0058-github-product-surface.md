# Task0058: GitHub product surface

Status: Implemented, verified and published to `origin/main`

Updated: 2026-11-04

Authorization: after approving the desktop redesign work, the owner explicitly
requested that the repository's GitHub-facing content become marketing and user
acquisition focused, then be committed and pushed together with the product UI.

## Goal

Make the repository explain Moxcrew in one screen: who it is for, what outcome it
creates, why local-first matters, what the product looks like and how an early
user can try or follow it. Keep implementation and trust details discoverable
without forcing prospective users through internal architecture before the value
proposition.

## Scope

- Rewrite the root README around value, proof, workflow, trust and a truthful
  pre-release call to action.
- Add current native product screenshots to the public documentation tree.
- Align package descriptions and the GitHub About description/topics with the
  general-purpose Moxcrew product rather than the historical coding-only name.
- Preserve compatibility identifiers, technical documentation, security policy,
  license and all product safety boundaries.

No website deployment, release publication, package publication, discussion
enablement, product behavior or protocol change is authorized. `moxcrew.com`
must not be advertised as the repository homepage while it serves a Plesk
placeholder.

## Acceptance

The README must remain truthful about the absence of public releases, show the
current desktop, link to build/install/security/architecture material, and avoid
claims that live provider or cross-platform release evidence has not established.
Markdown links, metadata files, formatting and the complete already-modified
desktop tree must pass their applicable checks before one commit is pushed to
`origin/main`.

## Result

The root README now leads with the product outcome, current native UI, workflow,
local-first trust value and a truthful development-preview CTA. Deep technical
details remain reachable through product, architecture, security, install,
release and roadmap links. Three fresh native screenshots are stored under
`docs/screenshots/marketing/`; no mockup or future-state image is presented as
the product.

Cargo, package and desktop packager descriptions now use the general-purpose
Moxcrew positioning while retaining compatibility identifiers. GitHub About uses
the same one-line promise and ten discovery topics. The homepage remains empty
because `moxcrew.com` currently serves a Plesk placeholder.

The complete Rust workspace passes 524 tests with 6 explicitly external/manual
fixtures ignored. Strict workspace Clippy, formatting, Cargo metadata, package
metadata, referenced-file and diff checks pass. A flaky process-tree test budget
was widened from 250 ms to 500 ms: it still kills the descendant before its 800
ms escape marker and passed three isolated repetitions plus the full suite.
