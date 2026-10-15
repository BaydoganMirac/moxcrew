# Desktop account protocol1

Status: Accepted for approved Task0033

Date:2026-10-15

This is browser-mediated installation sign-in, separate from operational V1/V2
and Memory protocol1. It does not make Local require an account or grant memory
sharing. The browser owns its existing GitHub/magic-link session; the native app
receives only a revocable paired-host credential. The approach uses the external
browser/loopback and S256 guidance in [RFC8252](https://www.rfc-editor.org/rfc/rfc8252)
and [RFC7636](https://www.rfc-editor.org/rfc/rfc7636), not an embedded login page.

## Browser request and callback

GET `/desktop/authorize` accepts exactly schemaVersion=1, installationId,
displayName, appVersion, protocolVersion=2, osFamily, redirectUri, state and
codeChallenge. Metadata follows existing pairing bounds. state is64 lowercase
hex characters; verifier is64 lowercase hex characters generated from independent
random UUID entropy, held only in native RAM. codeChallenge is43 base64url
characters: BASE64URL(SHA256(verifier)), without padding.

redirectUri is only `http://127.0.0.1:{port}/auth/callback`, canonical port1–65535,
no credentials/query/fragment. The native listener binds before browser launch.
The Cloud origin uses HTTPS; literal loopback HTTP is permitted only for tests /
local development under the existing transport policy. Cloud URLs are root
origins without credentials, paths or fragments.

The signed-out browser continues through existing sign-in and returns only to
this validated same-origin authorize path, using a protected ten-minute
continuation cookie. No arbitrary next/redirect destination is accepted. The
approval screen shows verified account, installation display name and a public
comparison code from state's first8 uppercase hex characters, formatted4–4.
POST `/api/desktop/approve` accepts the same exact fields plus action=allow|deny,
requires existing browser auth and same-origin bounded form validation. The
approval GET page uses Referrer-Policy same-origin so browsers preserve its
Origin on form submission; strict rejection of null/cross-origin POSTs remains.

Allow creates a five-minute grant in PostgreSQL and returns a static HTTP200
completion page that navigates to the exact loopback callback with code/state.
It uses meta-refresh and an explicit Return to Patchfleet link, with no scripts,
no-store and no-referrer. This preserves the site's same-origin form-action
policy: a cross-origin loopback redirect directly following the approval POST
can be blocked by browsers. code is43 random base64url characters; only its
SHA256 digest is stored. Deny uses the same completion page with
error=access_denied and state.
Only code/state OR error/state are valid callback fields. The native listener
requires matching state, GET, exact path/Host, no body, <=4096 request bytes,
bounded reads and <=16 rejected connections. It exposes only static no-store
HTML and closes after success, denial, cancel, shutdown or ten minutes.

## Token exchange and account

POST `/api/desktop/v1/token`, exact JSON:
`{schemaVersion:1,code,verifier,installationId}`. Use existing bounded JSON /
no-redirect transport, max8KiB and no-store. Validate current workspace, grant
expiry/consumption, original installation and S256 proof before atomic grant
consumption and existing host issuance. Legacy pairing cannot redeem this code.
Response is the existing pairing shape:
`{schemaVersion:1,hostId,workspaceId,credential,protocolVersion:2}`.
Single-use codes are not replayed after a lost response: start sign-in again;
the protected persisted installation ID revokes its prior same-workspace host.

GET `/api/desktop/v1/hosts/{hostId}/account`, existing Bearer credential, returns
exactly `{schemaVersion:1,hostId,workspaceId,accountId,email,subscription}`.
accountId is the verified owner's lowercase UUID; email is a bounded verified
account email or null. subscription is exactly
`{status,memoryAccess,periodEndsAt,checkedAt}`. status is
inactive|trialing|active|past_due|paused|canceled; memoryAccess requires stored
solo plan AND the current billing-derived entitlement. Timestamps use canonical
UTC milliseconds, periodEndsAt nullable. Account read itself never requires
payment. Unknown/revoked host, invalid credential or inactive/deleted workspace
is denied. Account data stays ephemeral and outside operational serializers.

POST `/api/desktop/v1/hosts/{hostId}/logout`, Bearer and exact
`{schemaVersion:1}`, revokes only that authenticated installation, returning
`{schemaVersion:1,revoked:true}`. Browser and other devices retain their sessions.
Native sign-out attempts this within3s and always removes its local credential;
remoteRevoked=false must be explained when offline. Downloaded and canonical
Local data are retained; they never become authoritative through sign-in.

All endpoints use no-store, safe uppercase DESKTOP_* errors and never log codes,
verifiers, credentials or URLs containing authorization query values. Browser
completion responses use no-referrer; the approval GET uses same-origin as
specified above. Fresh workspace/host/billing checks remain server
authority. Client account badges cannot bypass subscription or memory roles.

## Native actions and overlays

`cloud_login_begin {cloudUrl,displayName}` returns exact
`{schemaVersion:1,status:"waiting",authorizationUrl,expiresAt,confirmationCode}`.
`cloud_login_poll {}` returns the same waiting object, or
`{schemaVersion:1,status:"connected"|"denied"|"expired"|"cancelled"}`.
Errors use safe codes. Begin is idempotent for the same in-memory command ID;
pending state is intentionally not durable. Cancel is `cloud_login_cancel {}`.
`cloud_account_read {}` returns the wire account response. `cloud_logout {}`
returns `{schemaVersion:1,localSignedOut:true,remoteRevoked:<boolean>}`.
The ordinary Host snapshot remains free of account/token/callback details.
Bridge adds ephemeral cloudLogin/cloudAccount overlays scoped to the current
connection. Poll only while waiting, at most every3s, and stop on terminal state,
cancel, connection change or closing. Network actions observe shutdown and a3s
budget without holding the Local operations mutex. External browser opening is
triggered once after explicit begin; offer reopen/copy/cancel and clear errors.
