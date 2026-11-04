# Owner-visible system check

Updated: 2026-11-04

Status: Complete; locally verified

Plan: [Task0064](../plans/0064-owner-system-check.md)

The former Providers route is now an owner-facing **System check**. One explicit
action first runs the existing bounded connection diagnostic and then reads the
selected Project's existing Studio readiness. The desktop carries the exact
command identity between the two reads and never treats another Project's page
as completion.

The result keeps one hierarchy:

- local engine active;
- supported agent connections that can actually be attempted;
- installed production tools that passed their local version check;
- local image-engine availability; and
- configured Project app connections.

Missing production tools, the image engine and app connections are presented as
task-dependent setup rather than global failure. Direct actions open the existing
Connections, Project tools, image-engine or Apps surface. Provider observations
remain available below the owner summary and retain their separate refresh.

The check is non-durable and read-only. It never installs software, signs in,
runs a provider/model turn, changes an account, creates an artifact or executes
a remote app action. Saved credentials remain explicitly separate from model
entitlement, quota, tool permission and output-quality proof.

## Verification

- Exact sequencing, Project isolation, setup routing, Turkish copy and the
  800×560 first-action layout have focused native UI tests.
- The desktop package passed **240/240** tests with local loopback fixture
  permission.
- `cargo test --workspace --all-features -- --test-threads=1` passed **534
  tests, zero failures, six explicit opt-in tests ignored**.
- Strict workspace/all-target/all-feature Clippy, workspace formatting and
  `git diff --check` pass.
- The native dark Turkish
  [System check capture](../screenshots/system-check/system-check-tr-dark.png)
  is 2560×1760 physical pixels with SHA-256
  `0b5e96ccfd0e6ee1b1af46e40033c5b46f783a9ad8a6f182574eddac7a7e6aa2`.
  Its data is a test-only rendering fixture: no CLI, image engine or app account
  was probed.

Windows behavior remains unchanged and unverified. Live model access, provider
quota and representative output quality remain separate acceptance work.
