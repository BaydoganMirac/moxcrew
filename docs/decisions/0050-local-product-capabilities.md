# ADR0050: Local, conversation-first product capabilities

Status: Accepted by owner request; Task0052

Date: 2026-10-20

The owner approves rich inputs, visual revisions, image generation, video content
packages, brand profiles, browser work, taught skills, event triggers, parallel
alternatives, evidence and interactive outputs. Remote/phone task intake is
explicitly excluded. Keep one Chat entry and optional contextual controls.

Reuse the native Host, existing managed jobs/resources and typed production tools.
Optional owner-installed engines are bounded subprocesses or explicitly configured
local services; they are not application runtimes or new execution authorities.
Browser work uses a disposable isolated external browser and fixed operations,
never arbitrary agent JavaScript/shell or the owner's signed-in browser profile.
Interactive output is a checked file opened by deliberate owner action.

V14 capability facts extend the canonical append-only log for Project profiles,
proposed/accepted skills, trigger state/occurrences, variants and evidence. The
coordinator owns shared validation and snapshot integration. Full content remains
local by default; a scoped read/tool never widens an agent's bound Project.
Preserve existing event semantics, idempotency, expiry, immutable outputs and
frozen provider/file references. Old binaries require a stopped pre-upgrade backup
for rollback after V14 is written; do not rewrite canonical history.

Trigger arming is explicit bounded delegation for a frozen local recipe. It does
not authorize external publishing, purchases, messages or form submissions. A
sleeping/stopped Host does no work. Learned process data remains proposed until
owner acceptance; untrusted source instructions never become standing policy
merely because an agent extracted them. Evidence proves a quoted source binding,
not an automatic truth score.

Actual generation, OCR, speech and rendering require the corresponding engine.
Readiness and concrete errors must remain honest. Synthetic real-file protocol
proof establishes integration behavior; live model/account quality is a separate
kind of evidence and must be labelled accordingly.
