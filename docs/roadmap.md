# Patchfleet public roadmap

Status: active

Updated: 2026-10-15

This roadmap describes public technical direction, not dates or commercial
commitments.

## Now

- Apply the owner-selected [free Local/subscription Cloud boundary](decisions/0029-free-local-and-subscription-cloud.md)
  to the next memory and commercial implementation cards. Existing Cloud
  Free/Solo enforcement remains unchanged until its separately scoped transition;
  shared Cloud memory and pgvector are proposed, not implemented.
- Preserve the [local sourced ADR lifecycle](plans/0030-local-decision-lifecycle.md)
  on top of the completed Project/source foundation. Next implement explicit
  subscription Cloud publication under a separately approved concrete card,
  then evaluate Cloud vectors and the free Local index under Task 0029.
  Scoped agent access and automatic ADR drafts follow independently verified
  decision retrieval; owner acceptance remains explicit.
- Keep the completed Phase 1 observation, Phase 2 local control, and Phase 3
  paired Cloud control loops stable.
- Stabilize the shipped declarative agent catalog, owner-composed teams,
  bounded orchestrator, and capability-negotiated protocol V2.
- Collect the owner-operated production release evidence for the completed
  Plesk Cloud product: backup, migration, exact release SHA, readiness, pairing,
  remote action receipt, retention, billing, email, and rollback checks.

## Next

- Run the native desktop build/runtime matrix and signing evidence for
  [Task 0023](plans/0023-rust-desktop-migration.md). The macOS development
  application and DMG are locally built; native Windows/Linux runs remain.

- Validate the completed macOS, Windows, and Linux package, diagnostics,
  upgrade, uninstall, and recovery paths with external beta users.
- Validate the subscription Cloud product and the full Local-to-Cloud team
  journey with moderated external users after its commercial transition.
- Enable outbound notifications and Paddle entitlements in production only
  after operator configuration and provider sandbox evidence pass.
- Generalize execution control to Claude Code or Gemini only after a provider
  proves an equally safe supported lifecycle surface.

## Later

- Add organization collaboration only when real multi-user demand defines
  invitations, roles, and policy.
- Consider a provider SDK or self-hosted Cloud only when concrete demand
  defines its scope. The desktop shell is now owner-approved and implemented.
- Consider executable plugins only after two integrations cannot be expressed
  as declarative packs and a separate threat model is accepted.

The 2026-07-20 owner approval intentionally supersedes the earlier sequencing
gate that deferred packs and orchestration. ADRs 0019 and 0020 preserve the
original safety requirements while moving this work into the current phase.

Patchfleet remains useful without Patchfleet Cloud throughout every stage.
