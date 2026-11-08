# Task 0104: Adaptive failure recovery

Status: Implemented locally; live-provider usefulness remains a release gate

Updated: 2026-11-08

Authorization: after Task0103 hardening, the owner asked to continue with the
next product task. This card authorizes local repository code, tests and
documentation. It does not authorize a live provider call, deployment, release,
push, migration or deletion.

## Problem

An adaptive mission still failed immediately when any task reached a failed,
blocked or interrupted terminal state. A prepared evaluator therefore could not
inspect the failure or submit the bounded replan already supported by Task0103.
That made partial failure a terminal workflow rule instead of evidence for the
mission control loop.

## Contract

- Legacy and adaptive missions without a viable prepared reviewer retain the
  existing fail-fast behavior.
- An adaptive mission may keep running only while an existing `review` member is
  running or can directly evaluate its terminal dependencies.
- A recovery evaluator receives original Project inputs and completed upstream
  outputs. Failed output directories are never promoted to trusted inputs.
- V19 records the exceptional reviewer allocation after terminal dependency
  failure. It is valid only for the already prepared reviewer, provider, ability
  and budget.
- A bounded recovery replan may retire failed, blocked or interrupted work and
  writes a V20 checkpoint. Ordinary strict checkpoints remain V18; V17 and V11
  facts keep their original meaning.
- The replacement graph must remain schedulable, inside the prepared authority
  and inside existing task, job, elapsed-time and replan limits.

## Acceptance

- A failed producer does not immediately terminate an adaptive mission when its
  prepared reviewer can evaluate the failure.
- The reviewer can retire the failed branch and append a bounded replacement
  and follow-up evaluation task.
- Downgrading the recovery checkpoint to V18 fails replay.
- A retired failed task is not overwritten by its old work-item projection.
- Existing fail-fast, checkpoint, replay and desktop behavior remain green.

## Exclusions

No dynamic agent, provider, ability, credential, permission or tool acquisition.
No retry outside the existing mission budgets. No Cloud projection or remote
action. No claim that a fixture establishes live-model judgment quality.
