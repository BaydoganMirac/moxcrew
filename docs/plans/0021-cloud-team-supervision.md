# Task 0021: Cloud team supervision

Status: Approved

Owner approval: 2026-07-20

Depends on: Tasks 0019 and 0020

## User problem

An owner away from the development machine needs to understand whether a team
is progressing, blocked, waiting for approval, failed, or complete and take a
small set of safe actions.

## Product slice

- Add protocol V2 sanitized team, agent, task, question, approval, and receipt
  projections while keeping Local canonical.
- Add typed intents for team/agent cancellation, approval or rejection, and a
  bounded answer to a pending question.
- Add remote team start only from an existing opaque workspace alias, installed
  pack IDs, a bounded owner goal, and explicit local limits.
- Display team progress, freshness, pending decisions, outcomes, and an audit
  timeline in Cloud without source, paths, raw prompts, transcripts, tool output,
  environment data, or credentials.

## Acceptance criteria

1. V1 hosts continue to sync and Cloud advertises V2 actions only to compatible
   paired hosts.
2. Every remote action is authenticated, allowlisted, expiring, idempotent,
   auditable, locally revalidated, and proven by a terminal receipt.
3. Forbidden data is rejected by both Local and Cloud validators and covered by
   fixture and HTTP tests.
4. Offline, stale, rejected, expired, retrying, and terminal states are clear in
   responsive and accessible Cloud views.
5. Tenant isolation and paired Local/Cloud protocol tests cover every V2 intent.

## Rollout and rollback

V2 is capability-negotiated. Disabling it leaves the proven V1 projection and
remote cancel loop operational. Database rollback removes only rebuildable V2
projection and intent rows after a verified backup.
