# Desktop account sign-in

Status: Complete for approved Task0033; hosted/provider/platform release work remains

Updated: 2026-10-15

[Task0033](../plans/0033-desktop-account-sign-in.md) adds optional TR/EN native
sign-in, browser approval, account/subscription refresh and installation sign-out.
Local remains free, offline and usable without an account. The owner confirmed
that no live Cloud address exists yet; the server origin is configurable under
connection settings. No production URL is assumed.

The [account protocol](../desktop-account-protocol.md) reuses the existing browser
GitHub/magic-link session and protected installation credential. Rust opens an
ephemeral 127.0.0.1 callback, binds random state and an S256 proof, then uses the
system browser. Verifier, callback code/state, authorization URL and account data
stay in RAM and outside canonical facts, ordinary snapshots and operational sync.
The protected cloud.json schema remains unchanged. Browser provider sessions do
not enter the native app. This is an installation sign-in protocol, separate
from operational V1/V2, with no embedded browser or local Node service.

Cloud requires explicit authenticated owner approval and atomically consumes a
short-lived grant with host issuance. Account reads work without payment, but
Cloud memory continues to validate its own fresh plan, entitlement and roles.
Signing in never implicitly publishes Local ADRs or links shared projects.

The Bridge runs account requests in a separate cancellable lane so Local owner
commands and their original retry identity remain usable. Browser opening occurs
once per explicit begin; reopen/copy/cancel are available. Waiting polls run at
most every three seconds. Callback listeners and proof secrets close/clear on
completion, denial, cancellation, shutdown or ten-minute expiry. Network work has
a three-second aggregate budget. Fresh connection checks discard late responses.

Sign-out revokes only this host when reachable and removes its local credential;
offline revocation failure is explained. Local projects and downloaded ADRs stay.
An older sign-out cannot remove a newer connection. Legacy manual pairing remains
under advanced settings; UI sign-out uses the revocation-aware action.

## Verification

- Rust all-feature workspace:202 passed, three deliberate manual/private-fixture
  ignores. The new real cross-repository fixture was run separately and passed.
  Strict all-target/all-feature Clippy and formatting passed. All149 legacy
  reference tests passed. Private Cloud checks:61 passed/one intentional outer
  PostgreSQL skip; that original test passes inside the isolated upgrade child.
- Real Rust Host → approval HTTP → strict loopback callback → S256 exchange →
  disposable PostgreSQL passes. It covers wrong state before valid approval,
  unpaid account, restart, fresh paid subscription, no implicit sharing,
  protected credential mode, email/state/credential redaction, online logout,
  revoked access, denial and cancellation releasing the listener.
- Focused core checks cover timeout/expired/consumed codes, exact protocol/scope,
  cancellation of late installs, shutdown, bounded callback reads/rejections,
  offline sign-out, changed connection and continued Local commands during HTTP.
- A clean production Cloud build and compiled token/account/logout routes pass
  against disposable PostgreSQL without operator/provider secrets. The fixture
  simulates the existing trusted TLS proxy header; it is not a live TLS test.
- Real in-app Chromium browser form submission exposed Origin:null caused by
  no-referrer on the approval GET. The GET now uses same-origin, retaining strict
  Origin validation. The completion uses static HTTP200 meta-refresh/link and
  no-referrer to avoid form-action blocking cross-origin loopback redirects.
  Actual browser approval, navigation and S256 redemption then passed with a
  synthetic verified fixture identity. No real OAuth login is claimed.
- Actual native GL frames were captured through the real Host/HTTP fixture and
  inspected: [signed out](../screenshots/native-account-0.9-signed-out-tr.png),
  [waiting/comparison code](../screenshots/native-account-0.9-waiting-tr.png),
  [unpaid English800×560](../screenshots/native-account-0.9-free-en-min.png),
  [paid Turkish](../screenshots/native-account-0.9-paid-tr.png) and
  [paid English800×560](../screenshots/native-account-0.9-paid-en-min.png).
  These use synthetic identities/subscriptions and do not claim live accounts.
  They are renderer captures, separate from production OS presentation evidence.
- Independent review accepted core897e9ae, UI409b0fc and the final80d0793 fix.
  It found one cross-lane stale-snapshot race: retained normal worker messages
  could replace a fresh sign-in/sign-out snapshot. The bridge now refreshes a
  changed connection from the actual Host, preserving owner receipts/retries
  and avoiding an extra snapshot on ordinary same-connection ticks. A real
  callback/HTTP regression covers both transitions. Unlinked Local projects also
  no longer display a misleading red stale/offline cache warning.

## Native development artifact

Version0.9.0 feature-off macOS ARM64 App and DMG were built and verified. The
binary is12,680,832bytes, SHA256
`508c9f08b9a4362712103749a0b382ff85ead41a188e3198543382bdf97c9adc`.
The DMG is6,507,084bytes (6.21MiB), SHA256
`6549d00679f97fd6a83a4b1e804a5591ecd9579080e84abed3087ea22adc5a8b`.
Ad-hoc signature, bundle/version, five folder-purpose descriptions, no smoke
markers, native dynamic dependencies and DMG integrity passed. No model assets,
WebView or Node runtime are bundled. The external browser capability adds only
the existing eframe links feature and its small platform browser helper.

The exact packaged App was launched through NSWorkspace with its own disposable
data and provider stubs. Its own PID/bundle and1152×822 native window were
verified. The eight-second startup sample measured108784KiB RSS (106.23MiB) and
0.12s cumulative CPU. Normal close exited and released the writer lock. These
are short macOS measurements, not sustained/cross-platform resource guarantees.
OS window capture returned “could not create image from window”; the five
renderer frames above are separate evidence. No installed application or owner
data was replaced. Public signing/notarization remains release work.

Production Cloud deployment/migration, real GitHub/email authentication and TLS
configuration remain operator release work. Existing customer pricing changes
are separate. Windows/Linux native runtime/signing checks remain in the platform
release matrix.
