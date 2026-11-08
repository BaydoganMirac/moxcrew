# ADR 0057: Version strict adaptive checkpoint integrity

Status: Accepted; Task0103 hardening

Date: 2026-11-08

## Context

V17 introduced append-only adaptive checkpoint evaluations. Its command schema
described evidence, verdicts and graph revisions, but the Rust boundary still
accepted blank evidence, a non-satisfied verdict with every success criterion
marked passed, and a replan that retired a task while retaining a live dependent.

Tightening V17 replay in place would make an already accepted local fact invalid
after upgrade. That would violate the append-only event contract even when the
old fact represented a poor model decision.

## Decision

New adaptive checkpoint evaluations use schema V18. The command boundary and
V18 replay require:

- non-blank evidence for every evaluated criterion;
- `satisfied` only when every criterion passed;
- `replan` or `blocked` only when at least one criterion remains unmet; and
- no pending or queued task dependent on retired work after task retirement and
  addition.

V17 facts retain their original validation and replay meaning. They remain
readable and auditable, but no new writer emits them. V18 changes no provider,
permission, budget, Project scope or Cloud payload.

## Consequences

- A poor checkpoint cannot convert blank or contradictory assertions into a
  successful adaptive mission.
- Replanning must retire affected dependents and add a valid replacement chain
  instead of leaving pending work permanently attached to a retired task.
- Rollback after the first V18 fact requires the complete stopped pre-upgrade
  data backup; event lines must never be rewritten to V17.
