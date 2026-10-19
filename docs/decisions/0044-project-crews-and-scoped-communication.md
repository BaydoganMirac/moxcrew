# ADR 0044: Project crews and scoped communication

Status: Accepted

Date: 2026-10-19

## Decision

Moxcrew's central interaction is conversational orchestration. Mox recommends
an editable crew from the requested outcome; owners explicitly start work and
can reuse a crew in its Project. Projects retain context/decisions, crews retain
role composition, and missions retain individual goals, tasks, messages and
deliveries. Small requests may still use a single agent.

Implement the Project orchestration domain as V10 local facts alongside legacy
workspace-bound teams. Reuse managed jobs and provider control rather than
inventing another process runner or changing legacy team event meanings. Crew
communication uses the existing authenticated run-scoped transport with new
explicitly scoped tools; ordinary work gains no crew mutation authority.

Messages are attributable, bounded work exchanges. The Host resolves sender
identity and rejects cross-Project recipients. A durable message does not mean
a recipient ran or answered. Show actual task and delivery state. Bundled-guide
recommendations identify their deterministic basis and remain editable.

All goals, member instructions, messages and artifacts stay local under the
existing Cloud boundary. No new hosted capability or provider execution support
is implied. Task0043 supplies the implementation and verification evidence.
