# ADR 0040: Version 1.0.0 release baseline

Status: Accepted release scope; implementation and release evidence pending

Date: 2026-10-19

## Context

ADR 0039 selects one broad general-purpose release. The owner subsequently
accepted the proposed ability plan and asked for it to be ready for version
1.0.0. The target must survive handoffs without becoming a narrow launch or an
unsupported claim that the current application already implements the plan.

## Decision

Set Moxcrew 1.0.0 as the target for eight capability areas, forty defined
abilities and ten composed project workflows. The
[master release plan](../plans/0039-version-one-release.md) owns the public
technical scope, dependency order, acceptance matrix and evidence requirements.
The private repository owns accepted launch messaging and launch preparation.

This is the approved product/release baseline. Routine implementation planning
must retain it. Concrete tool/provider choices, supported formats, execution
environment and estimates need technical evidence and bounded cards; the scope
approval does not establish those answers or a release date. Do not silently
defer a whole capability area past 1.0.0 or reduce the release to three abilities.

Existing local-first, native UI, explicit access/sharing, free Local and optional
subscription Cloud boundaries remain. An incompatible execution-environment
choice still requires a public architecture decision. Nothing here authorizes
new billing, secret creation, hosted execution, deployment or publication.

## Release truth

1.0.0 is a target, not the current package version or a completed milestone.
Keep source/package versions and compatibility claims unchanged until a reviewed
release candidate is built. Each ability must produce and revise a verified
output or apply a supported action; the composed workflows must work end to end.
The native installation/platform, recovery, privacy and commercial release
checks remain necessary. Partial evidence is recorded as partial.

## References

- [ADR 0039](0039-broad-general-purpose-ability-release.md)
- [Master release plan](../plans/0039-version-one-release.md)
- [Release state](../state/version-one-release.md)
