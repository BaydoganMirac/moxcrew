# ADR0034 Native browser account sign-in

Status: Accepted for approved Task0033

Date:2026-10-15

Add optional account sign-in to the native desktop. Keep Local usable without
an account. Reuse the current Cloud identity, billing authority and installation
credential instead of introducing a second native password or provider session.

Use the system browser with a strict ephemeral loopback callback, random state
and S256 proof. This follows native browser guidance without adding a WebView,
Node sidecar or platform URL-scheme registration. Enable the existing eframe
links capability for browser opening. Authentication callbacks cannot execute
actions. Short-lived single-use grants stay in Cloud PostgreSQL; the app stores
only its existing protected installation identity and paired credential.

Expose verified account/subscription state as ephemeral UI data. Memory access
continues to be checked by Cloud on every request. Sign-out keeps Local state
and downloaded ADRs, revokes only this host when reachable, and explains offline
revocation. Browser OAuth configuration and actual Cloud deployment are separate
operator release steps. See the [desktop account protocol](../desktop-account-protocol.md).
