# Moxcrew public roadmap

Status: active

Updated: 2026-10-19

This roadmap describes public technical direction, not dates or commercial
commitments.

## Now

- Preserve the completed [Moxcrew identity and native interface](state/dobbly-brand-interface.md),
  with Mox as the mascot and compatible existing installation/data identities.

- Preserve the owner-approved [studio interface](plans/0036-product-studio-interface.md)
  with native TR/EN, adaptive project/team/knowledge screens and the existing
  lightweight runtime.

- Preserve optional desktop account sign-in, account/subscription reads and
  installation sign-out in [Task0033](plans/0033-desktop-account-sign-in.md).
  The Cloud origin is configurable until an operator provisions a live service;
  verified browser provider/TLS configuration remains release evidence.

- Apply the owner-selected [free Local/subscription Cloud boundary](decisions/0029-free-local-and-subscription-cloud.md)
  to the next memory and commercial implementation cards. Existing Cloud
  Free/Solo enforcement remains unchanged until its separately scoped transition;
  new shared memory requires verified paid entitlement. Cloud pgvector retrieval is implemented under Task0032; the broader existing-customer commercial transition is separate.
- Preserve the [local sourced ADR lifecycle](plans/0030-local-decision-lifecycle.md)
  on top of the completed Project/source foundation and explicit subscription
  Cloud publication in [Task 0031](plans/0031-cloud-memory-publication.md).
  Preserve the measured Cloud retrieval and free Local keyword search in
  [Task0032](plans/0032-project-memory-search.md).
  Scoped Local agent access is implemented in [Task0034](plans/0034-scoped-agent-memory.md).
  Optional consented ADR drafts are implemented in [Task0035](plans/0035-automatic-sourced-drafts.md); owner acceptance remains explicit. Cloud agent attachment is the next separate memory slice.
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

Moxcrew remains useful without Moxcrew Cloud throughout every stage.
