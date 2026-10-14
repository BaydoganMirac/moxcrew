# ADR 0025 Project identity and owner source foundation

Status: Accepted for Slice A

Date: 2026-10-14

Owner approved the proposed Project, durable source capture and native Project
Memory package. ADR 0021 still governs future decision acceptance and agent access.

A Project has a stable opaque identity independent of a Git path. Owners create
projects and explicitly link registered workspaces; one workspace belongs to at
most one project. Unlinking or removing a workspace preserves historical memory.
Existing workspaces remain usable without a project and are never grouped by
guessed path, repository name or remote URL.

Version 3 local events extend the canonical log without changing strict V1 work,
workspace or team records, or the public Cloud protocol. Owner commands have
durable requests and terminal receipts. Project messages are append-only source
facts with optimistic revision checks; edits preserve the original event history.

Explicit versioned subject bindings freeze the project of locally submitted work
and teams before their creation facts. Replay derives task instructions, team
goals, owner answers and approval notes from those facts. Generated team prompts
are not bound as owner sources. A retried task inherits its original binding.
This avoids a second ingestion writer and a crash gap between a fact and its
source. The binding alone never creates a source or an accepted decision.

Source IDs, origin event IDs, kind, project, actor and revision provide provenance.
Captured task/team text is immutable; only directly entered project messages can
be edited. Sources are context, never accepted ADRs. Remote Cloud team creation
and external provider transcripts do not gain implicit source capture in this slice.

The existing protected event log remains authoritative. A sixth derived projection
holds project/source state and can be rebuilt. Desktop snapshots carry project
metadata; bounded project-specific reads supply up to 20 sources, with explicit
keyword filtering. Full history is not cloned into every desktop refresh or Cloud
payload. Existing Cloud serializers remain allowlists and omit all memory data.

No database, embedding dependency, model request, MCP endpoint, source deletion
or automatic extraction is introduced. Those belong to the independently reviewed
slices in Task 0022.
