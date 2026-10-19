# ADR 0038: General-purpose projects

Status: Accepted product direction; broader execution support is not implemented

Date: 2026-10-19

Owner direction: retain Project as the main concept, reject the proposed
Workspace approach, and make Moxcrew useful for work beyond software development.

## Decision

Moxcrew helps people turn goals into outcomes with AI agents, organized in
projects with persistent context, tasks, decisions and results. Software
development is one use case alongside research, writing, analysis, planning
and personal work. The product is not restricted to developers or coding agents.

A Project is the durable scope of an ongoing goal or body of work. It is
independent of a repository, provider, conversation, folder and temporary run.
Keep Project as the primary user-facing container; do not require a separate
Workspace entity or hierarchy in the basic product flow. Existing internal
workspace identifiers remain implementation details with their present meaning.

Projects can conceptually organize a goal, owner-provided context/resources,
tasks, agent work, results and accepted decisions. These are domain concepts,
not claims that file ingestion, arbitrary outputs or non-coding execution have
already shipped. Repository links remain useful for coding projects but do not
define what a project is.

Decisions are owner-accepted choices within a project, not exclusively software
architecture records. For example, a research project's evidence criteria or a
campaign's selected audience can be project decisions. Notes, ideas, sources,
temporary task constraints and generated suggestions do not become binding
decisions automatically. Preserve explicit acceptance, provenance and history.
This direction introduces no global decision inheritance. Existing ADR schema
names and immutable facts keep their meaning; user-facing language should be
plain Decisions rather than requiring knowledge of ADR terminology.

The intended basic interaction is goal-led: describe the project and desired
outcome, inspect the proposed work and required access, authorize supported
actions, then review progress and results. Do not require Git, branch, terminal
or provider-protocol knowledge for general-purpose usage. Do not invent a
required resource or permission when the requested outcome does not need it.

## Delivery truth and constraints

Today project creation and owner-entered memory/decisions already have stable
identities independent of folders. Actual task/team execution still depends on
registered Git worktrees and supported Codex control; Claude/Gemini are observed
rather than generally controlled. The delivered Quick start still reflects that
coding-oriented implementation. General-purpose execution, plain-folder/file
ingestion, browser or application actions and an adapted onboarding require
separately bounded approved implementation cards and verification.

Do not present example research, content or planning outcomes as capabilities
already verified end to end. Select and prove a concrete non-coding outcome
before claiming that broader runtime exists. The first such slice remains
unselected; this is not approval to build every tool or integration at once.

Keep the single local authority, attributable bounded actions, capability and
consent checks, free account-optional Local and optional subscription Cloud.
Broader audiences do not authorize additional data upload, arbitrary remote
shell, unbounded autonomy or automatic external publication. Cloud sharing stays
explicit under the existing separate protocol. No runtime, schema, migration,
provider setting, deployment or publication change follows from this record.

## Relationship to earlier records

This supersedes developer-only audience/positioning and the assumption that a
user's project is necessarily a coding repository. It preserves the stable
Project and explicit memory authority in ADRs 0021/0025/0031/0035, the local-first
boundary and ADR 0029's commercial direction. Existing Git workspace contracts
remain valid for the currently implemented coding execution path.
