# Task0033 Desktop account sign-in

Status: Complete and independently accepted; hosted/provider/platform release checks remain

Owner approval:2026-10-15, “O zaman masaüstüne login vs koymamız lazım”.

Ship an optional native TR/EN account entry point, browser sign-in, account /
subscription status, cancellation and sign-out. Local remains free, offline and
usable without an account. Reuse Cloud's current GitHub/magic-link browser auth
and protected paired-host credential; do not embed a browser or provider session.
No live deployment, customer migration, purchase or new pricing is included.

## Contract

The native app generates a random verifier and state in RAM, opens one ephemeral
127.0.0.1 listener and starts the external browser on its selected Cloud origin.
The public Desktop account protocol1 specifies an S256 proof challenge, strict
loopback callback, browser approval, short-lived single-use authorization code
and protected token exchange. No secret verifier, Cloud credential, password or
provider token is placed in a URL. Account sessions stay in the browser.

The callback is an authentication response only, never a command ingress.
Validate state, exact path/query/Host, size and time bounds before accepting it.
Ten-minute waiting, explicit cancel/restart/close and three-second network
budgets must release the listener and keep Local commands responsive. Persist
the existing installation identity before launching; the existing credential
rotation recovers an orphaned response through a new sign-in. Pending verifier /
code / state are not written into events, snapshots or files.

Cloud approval requires an authenticated owner, an explicit decision and same-origin
validation. A short-lived code can be redeemed only once and cannot be used by the pairing
endpoint.

An authenticated account read returns minimal verified owner identity and fresh
server-derived subscription state, even when no paid entitlement exists. It is
ephemeral UI information; memory endpoints still independently check paid access.
Sign-out revokes this installation when online and always clears its local
credential if the owner requests it. Explain offline revocation failure; keep
Local projects, providers and downloaded ADRs. Legacy manual pairing stays under
advanced connection settings. A real default Cloud address is optional owner
input; do not invent a production endpoint or read live environment secrets.

The owner confirmed that the address should remain configurable because no live
Cloud exists yet. Verification, review fixes, actual native frames and the0.9
development artifact are recorded in [account state](../state/desktop-account.md).

## Ownership

- Coordinator: this plan, public account protocol/ADR, shared indexes, Cargo
  dependency/features/version, main.rs smoke, cross-repo fixtures/proofs, final
  docs/state and feature-off native package.
- Rust builder: new core desktop_login.rs and tests, lib.rs/host.rs/cloud.rs
  integration only. No Cargo/desktop/private edits. Bounded native callback,
  PKCE, ephemeral status/account reads, cancellation and protected pairing.
- Native builder: ui.rs/i18n.rs/bridge.rs/command.rs only. Account entry in sidebar
  and Cloud, browser launch through eframe links, explicit consent/recovery and
  ephemeral scoped overlays. No main/Cargo/core/private edits.
- Independent review after stable commits: auth/state/PKCE/code replay,
  cross-workspace isolation, redaction, cancellation/revocation, UI scope/races.

## Acceptance

Use fresh synthetic fixtures only. Prove successful browser approval→real Rust
callback→HTTP exchange→restart/account read; denied, stale, wrong state, wrong
verifier, consumed/expired code, arbitrary redirect, wrong host/workspace,
revocation and changed subscription. Test missing browser/retry/cancel/outage /
close without blocking Local or leaking secrets. Existing events/protocols and
memory tests remain valid. Inspect actual TR/EN native frames at minimum size,
run affected checks and independent review, finish with a local reviewable
commit and development package. Operator OAuth/provider/TLS deployment remains
a separate release step.
