# ADR 0039: Broad general-purpose ability release

Status: Accepted product/release direction; detailed implementation pending

Date: 2026-10-19

## Context

ADR 0038 retains Project and expands the intended audience beyond developers.
The owner then rejected a proposal to launch only three brief/research/content
abilities and explicitly requested a much broader feature set in one release,
including presentation creation and video editing. Existing skill ecosystems
should be evaluated for reuse rather than treating every workflow as a new
implementation from scratch.

## Decision

Plan one broad general-purpose capability release. Presentation creation and
editing, actual video editing and research are core scope. Evaluate document,
image, audio, data, web and connected-action capabilities as part of the same
release definition. Support composed work that carries project sources and
outputs across the relevant abilities, with Mox coordinating the crew.

Bounded implementation cards and dependency ordering remain necessary inside
the release. They do not imply a three-ability public launch or authorize
silently narrowing the owner-selected broad direction. Detailed capability
acceptance, supported formats, dependencies, provider access and release timing
remain to be defined. This does not claim these capabilities currently work.

Prefer existing skill formats and proven tools where compatible with the
product. A skill's instructions do not supply its runtime, model access or
permissions. Establish provenance, licensing and compatibility before bundling
or enabling a package. No universal third-party skill compatibility is assumed.

## Preserved boundaries

Project remains the main scope, without a required Workspace hierarchy. Native
Rust/egui, the single local writer, explicit decisions, bounded commands,
capability checks, free account-optional Local and optional subscription Cloud
remain in force. Existing Git validation stays intact until a supported
non-Git execution contract replaces that path for general-purpose work.

Any execution-environment choice that changes the native runtime contract needs
its own public architecture decision. This record does not add Node, an embedded
browser, an executable plugin marketplace or hosted rendering to the product.
Provider access/costs, explicit sharing and external-action authorization are
not widened by feature breadth. Cloud remains a control plane.

## Consequences

- Public product and roadmap must reflect the broad release direction rather
  than the superseded narrow-launch recommendation.
- Implementation work still needs concrete owned files and runnable acceptance
  checks. Output files and applied actions need real verification.
- Current coding execution remains valid; unsupported capabilities are not
  marketed as delivered. No runtime, schema, protocol or deployment changes
  follow from this documentation decision.
- Detailed ability catalog, launch messaging and commercial research stay in
  the private repository under ADR 0007.

## References

- [ADR 0038](0038-general-purpose-projects.md)
- [Product](../product.md)
- [Roadmap](../roadmap.md)
