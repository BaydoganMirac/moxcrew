# ADR0052: Explicit frozen handoffs in reusable recipes

Status: Accepted for owner-authorized Task0099

Date: 2026-11-05

## Decision

Reusable workflow recipes may carry one optional `memberDependencies` array
aligned with their ordered members. Each member may depend only on unique earlier
member indices. The restriction makes the recipe graph bounded and acyclic
without a second workflow language. Recipe preparation maps those indices to the
existing mission task IDs; the existing scheduler then waits for completed work
and copies verified upstream files through its current input-manifest contract.

The dependency plan is frozen with saved and scheduled recipes. Preparing work
still requires an owner command and creates a prepared mission; starting that
mission remains a separate explicit action. Dependencies do not grant provider,
tool, account, network or side-effect authority.

Legacy recipes without explicit dependencies retain the accepted research,
producer and review phase derivation. Saving a mission records members in task
order plus its exact dependency indices so reuse does not silently flatten the
workflow.

## Compatibility and projections

The field extends the existing local V11 recipe fact and remains optional. Public
Cloud serializers and remote intents do not expose it. Routine native metadata may
show the frozen handoff labels needed for owner review. Event replay recomputes the
expected task graph from the owner-selected recipe revision instead of trusting an
arbitrary prepared graph.

As with the other post-baseline local event extensions, owners must keep an
unchanged stopped backup before opening adopted logs with older binaries. No new
database, workflow service or persistent sidecar is introduced.
