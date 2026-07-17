# Task card 0001: Secure local app shell

Status: Completed

Coordinator: Patchfleet coordinator

Builder: one implementation owner after approval

Reviewer: independent reviewer after the builder commit

Updated: 2026-07-15

Approved by owner: 2026-07-15

Review amendment approved by owner: 2026-07-15

Completed: 2026-07-15

Independent review: Passed

## Objective

Make the existing Next.js application a safe local-only foundation before it
can observe or control provider processes.

The observable outcome is that development and production commands listen on
the IPv4 loopback interface, requests with an untrusted Host header fail
closed, and browser responses carry a small baseline of local-console security
headers.

## Owned files

The builder may change only:

- `package.json`;
- `next.config.ts`, or its dependency-free JavaScript replacement;
- `middleware.ts` or `middleware.js`;
- `app/layout.tsx`;
- `app/page.tsx`;
- `app/globals.css`;
- one test file under `tests/`;
- `README.md`;
- `docs/state/v0-foundation.md`.

Any additional file requires coordinator approval before editing.

## Inputs

Treat these as read-only contracts:

- `AGENTS.md`;
- `docs/product.md`;
- `docs/architecture.md`;
- `docs/plans/v0-local-first-control-loop.md`;
- ADRs 0001–0008.

## Required behavior

1. `npm run dev` and `npm run start` bind explicitly to `127.0.0.1`.
2. For browser-originated requests, the application boundary accepts only
   `localhost` and `127.0.0.1` Host values, with an optional valid port, and
   rejects missing, malformed, or other Host values exposed to middleware.
3. Responses produced by the middleware boundary set
   `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
   `Referrer-Policy: same-origin`, and a restrictive `Permissions-Policy` for
   camera, microphone, and geolocation.
4. The page identifies itself as a local console without fake provider,
   runtime, Cloud, account, or billing state.
5. The implementation uses Next.js features and the Node.js standard library;
   it adds no dependency, custom server, daemon, or desktop wrapper.
6. One runnable Node test covers the loopback command configuration, accepted
   Host values, rejected Host values, and security headers.

## Acceptance

- `npm test` succeeds on a clean checkout after `npm install`.
- `npm run build` succeeds.
- A production smoke run serves the page at `http://127.0.0.1:3000`.
- The smoke response contains every required security header.
- A browser-style request with an untrusted Host header receives a rejecting
  status.
- The application makes no outbound network request.
- `git diff --check` succeeds.

## Forbidden scope

- provider discovery or adapters;
- agent, session, run, or work-item data;
- event logs, projections, or persistence;
- API routes or mutating controls;
- Cloud pairing, sync, authentication, or billing;
- Claude Code, Codex, or Gemini process execution;
- analytics, telemetry, desktop packaging, or background services;
- new runtime or test dependencies.

## Handoff

The builder must update `docs/state/v0-foundation.md`, create one focused local
commit, and report the commit hash, changed files, checks, and remaining risks.
The builder must not push.

The reviewer starts only after that commit exists. The reviewer checks the
acceptance criteria, local-only boundary, dependency diff, and forbidden scope,
then reports findings without rewriting the builder commit.

## Deferred security work

Authentication, CSRF protection, and a nonce-based Content Security Policy are
not useful in a read-only shell with no API or sensitive projection yet. They
must be reconsidered before the first mutating endpoint, non-loopback listener,
or untrusted content is introduced.

Raw duplicate Host headers and absolute-form request targets that Node.js or
Next.js handles before middleware are outside this browser boundary. ADR 0008
records why a custom HTTP server is not added for those forms.
