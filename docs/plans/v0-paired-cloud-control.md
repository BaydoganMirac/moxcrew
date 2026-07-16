# Phase 3: Paired Cloud visibility and remote cancel

Status: Done

Owner: Patchfleet coordinator

Approved by owner: 2026-07-16

Updated: 2026-07-16

## Outcome

One owner can pair a local Patchfleet installation with the private Cloud,
observe a sanitized operational projection from another browser, issue one
typed `cancel_run` intent, and see the definitive local receipt. Revocation,
replay, expiry, stale revisions, restart, and Cloud failure fail closed.

## Delivery sequence

1. [Task 0011](0011-public-cloud-sync.md) freezes the version-one wire shapes,
   sanitizer, credential file, pairing UI, polling loop, local intent
   application, and public contract tests.
2. The private Cloud task implements owner authentication, durable pairing,
   host authorization, projections, cancel intents, receipts, revocation, and
   the remote console.
3. A cross-repository smoke proves pairing, sync, remote cancel, receipt return,
   revocation, and offline local operation.
4. Independent security and simplicity reviews pass before closure.

## Locked scope

- one owner and one workspace;
- multiple paired hosts;
- manual pairing and disconnect;
- automatic outbound heartbeat/projection/intent polling;
- allowlisted current operational projection only;
- remote `cancel_run` only;
- local and Cloud JSON storage only for this single-node, undeployed alpha;
- no new runtime dependency in either repository.

## Done gate

- Plain pairing codes and host credentials are never stored by Cloud.
- Every host API authenticates the exact host and rejects revoked credentials.
- Forbidden-field canaries cannot enter a request, Cloud state, response, log,
  receipt, or rendered page.
- Stale projection revisions, message replay, cross-workspace access, duplicate
  intents, conflicting receipts, unsupported versions, and expired commands
  fail closed.
- A dropped receipt response is safely retried without another local provider
  side effect.
- Cloud absence or failure leaves the Phase 2 local console fully usable.
- Both repositories pass tests, production builds, and diff checks.
- The final two-repository artifact has no unresolved P0-P2 review finding.
- All commits remain local; nothing is pushed or deployed.

## Deferred

- remote enqueue, questions, revise/reorder/remove, and provider start;
- workspace alias mapping;
- production auth, migrations, deployment, background jobs, billing,
  notifications, history, teams, and packaging;
- WebSockets, SSE-only delivery, terminal/log/source/diff display.

## Completion evidence

- Public suite: 128/128 tests passed.
- Public and private production builds passed.
- Real loopback HTTP smoke passed pairing, projection, remote cancel, durable
  receipt, revocation, and post-revocation denial.
- Independent cross-boundary re-review reported no remaining P0-P2 finding.
- Both repository diffs passed whitespace validation; no dependency or
  lockfile change was introduced.
