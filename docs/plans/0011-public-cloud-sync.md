# Task 0011: Public host pairing and Cloud sync

Status: Done

Coordinator: Patchfleet coordinator

Builder: public host-sync owner

Reviewer: independent cross-boundary reviewer; completed with no P0-P2 finding

Approved by owner: 2026-07-16

## Objective

Add the public half of the Phase 3 loop without weakening local-only operation:
pair, sanitize, heartbeat, publish, poll `cancel_run`, apply through the existing
command engine, return the receipt, and retry safely.

## Owned files

- `lib/cloud/` and focused Cloud-sync tests;
- one Cloud mutation route and one internal polling route under `app/api/`;
- the Cloud status/forms in `app/page.tsx` and minimal responsive CSS;
- `scripts/local-next.mjs` for the bounded loopback poll trigger;
- public protocol, ADR, plan, and state files reserved by the coordinator.

Provider adapters, Phase 2 schemas, dependency manifests, Gemini extension,
middleware trust rules, and private Cloud files are read-only unless the
coordinator explicitly integrates a reviewed contract fix.

## Acceptance

- sanitizer is an allowlist and drops titles, instructions, paths, provider
  native data, prompts, transcripts, source, diffs, tokens, and credentials;
- credential storage is atomic and mode 0600;
- Cloud URL is HTTPS except exact loopback HTTP;
- sync validates response versions, host identity, cursor ordering, intent
  schema, expiry, capability, revision, and idempotency;
- local receipts are retried unchanged and cursor advances only after Cloud
  acknowledges them;
- no paired configuration or failed Cloud request blocks local operation;
- tests, production build, diff check, and a cross-repo smoke pass;
- no dependency or lockfile change and no push.

## Forbidden scope

- arbitrary remote commands or shell text;
- remote enqueue/workspace paths;
- daemon, desktop shell, installer, auth provider, telemetry, or Cloud SDK;
- provider-control changes.

## Completion

The bounded host slice, contract tests, production build, and cross-repository
HTTP control-loop smoke passed on 2026-07-16. No dependency, lockfile, push, or
deployment change was made.
