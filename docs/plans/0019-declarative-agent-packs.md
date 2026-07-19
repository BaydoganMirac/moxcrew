# Task 0019: Declarative agent packs

Status: Approved

Owner approval: 2026-07-20

## User problem

An owner should not rewrite the same role prompt, provider choice, permissions,
and quality expectations every time work starts.

## Product slice

- Ship a versioned, declarative agent-pack manifest with strict validation.
- Include built-in packs for orchestration, product, design, frontend, backend,
  full-stack, QA, review, security, release, documentation, and research.
- Let the owner inspect, install, update, and remove local custom packs without
  loading executable code.
- Keep pack instructions, local paths, and provider credentials on Local.
- Expose pack provenance, capabilities, permissions, and compatibility before
  installation or use.

## Acceptance criteria

1. Invalid, oversized, unknown-version, duplicate, or path-escaping packs fail
   closed before they can be installed.
2. Built-in packs work without setup and cannot be silently overwritten.
3. A custom JSON pack can be installed, listed, selected, updated, and removed
   through supported Local surfaces.
4. Pack instructions and permissions are never included in a Cloud projection.
5. Domain, registry, API/CLI, UI, packaging, and negative-boundary tests pass.

## Rollout and rollback

The registry is local and additive. Removing a custom pack leaves historical
team snapshots readable. Rollback removes the new surfaces and ignores pack
events without changing existing work, workspace, or observation projections.

## Deliberately excluded

Executable plugins, downloaded JavaScript, arbitrary tools, a marketplace, and
provider credentials inside a manifest require a separate threat model and ADR.
