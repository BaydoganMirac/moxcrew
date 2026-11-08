# ADR 0058: Let prepared reviewers evaluate terminal adaptive failures

Status: Accepted; Task0104

Date: 2026-11-08

## Context

The scheduler historically terminated a mission as soon as any task failed.
Task0103 introduced bounded evaluation and replanning, but a reviewer whose
dependency failed could not be allocated because V11 allocation requires every
dependency to have completed successfully. Reinterpreting V11 or V18 would
break append-only replay.

## Decision

Keep existing event meanings and introduce two recovery-specific versions:

- V19 allocates only an already prepared `review` member when at least one
  direct dependency has durably failed or been interrupted and every direct
  dependency is either completed or terminal unsuccessful.
- V20 records a strict checkpoint that retires terminal unsuccessful tasks and
  appends a bounded replacement graph. V18 remains the ordinary strict
  checkpoint version.

The scheduler waits only while such a prepared evaluator remains viable. Failed
outputs are not copied as trusted inputs. The reviewer receives no new provider,
ability, tool, permission, credential or budget. Retired tasks retain their
historical work identity and are not overwritten by the work projection.

## Consequences

- A partial provider failure becomes inspectable evidence instead of an
  unconditional mission failure when the approved plan already contains a
  reviewer.
- Missions without a viable reviewer retain fail-fast behavior and bounded
  timeouts still terminate stalled evaluation.
- Readers without V19/V20 support reject the new log; rollback requires the
  complete stopped pre-upgrade backup rather than event rewriting.
