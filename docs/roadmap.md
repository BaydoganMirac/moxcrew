# Moxcrew public roadmap

Status: active

Updated: 2026-10-25

This roadmap describes public technical direction, not dates or commercial
commitments.

## Now

- Validate the delivered [native UX rules implementation](state/ux-rules-implementation.md)
  with real users and assistive technology. Task0053 implements removable input
  chips, output filter chips, a stable composer and contextual actions; desktop
  tests and native fixture captures establish the bounded rendering/flow evidence.

- Continue live-model and platform release evidence for the delivered
  [eleven local product capabilities](plans/0052-eleven-product-capabilities.md):
  rich inputs, visual revisions, image/video production, Project style, browser
  work, taught skills, bounded triggers, alternatives, source evidence and
  interactive outputs. Keep the main Chat composer simple and provider selection
  optional. [State and evidence](state/eleven-product-capabilities.md) distinguish
  real media/browser proof from protocol peers and unverified live models.

- Validate the delivered [productive Project crews](plans/0044-productive-project-crews.md):
  model planning, mixed providers, output revisions, typed production, live
  direction, cross-crew delivery/review, resources, recurring recipes, explicit
  app actions and actual usage. Track evidence and limits in
  [Task0044 state](state/productive-project-crews.md).

- Execute the owner-approved [1.0.0 master release](plans/0039-version-one-release.md):
  eight capability areas, forty abilities and ten composed Project workflows
  in one release. The [Create studio](state/project-create-studio.md) implements
  managed non-Git jobs, recipes and native result reads/export. Task0044 adds
  frozen resources and bounded Office/data/media operations. Track real output and integration
  evidence in [release state](state/version-one-release.md); no date or current
  1.0.0 completion is implied.

- Deliver the [owner-connected provider/default-skill experience](decisions/0042-owner-providers-default-skills-and-motion.md):
  guided local Codex/Gemini CLI/Claude Code setup, one provider sufficient,
  checked default outputs and revisions, and purposeful Mox/crew motion.
  [Task0041](state/connected-project-conversation.md) adds persisted Project
  conversation, safe file follow-ups, connection diagnostics, bundled guides
  and reduced motion. Task0044 implements Claude/Gemini control with protocol
  peers; live provider/account and complete ability quality evidence remain.

- Apply the owner-selected [general-purpose Project direction](decisions/0038-general-purpose-projects.md)
  to future product work. Keep Project as the main container; research, content,
  analysis and planning belong in the target audience alongside coding. Current
  Create supports non-Git work through selected local providers; ordinary coding tasks/teams and
  their explicitly opened Quick start retain the Git workflow.

- Preserve the completed [Moxcrew identity and native interface](state/dobbly-brand-interface.md),
  with Mox as the mascot and compatible existing installation/data identities.
  [Task0042](state/futuristic-native-studio.md) refines the native conversation
  into an ink/cobalt studio with a centered Mox welcome and adaptive cards.

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

- Complete the approved 1.0.0 ability/composition acceptance matrix, including
  presentations, video editing, research and the other five areas. Keep skill/tool
  compatibility and execution-environment decisions explicit; do not substitute
  a three-ability launch or remove Git validation before replacement contracts
  are verified.

- Run the native desktop build/runtime matrix and signing evidence for
  [Task 0023](plans/0023-rust-desktop-migration.md). The macOS development
  application and DMG are locally built; native Windows/Linux runs remain.

- Validate the completed macOS, Windows, and Linux package, diagnostics,
  upgrade, uninstall, and recovery paths with external beta users.
- Validate the subscription Cloud product and the full Local-to-Cloud team
  journey with moderated external users after its commercial transition.
- Enable outbound notifications and Paddle entitlements in production only
  after operator configuration and provider sandbox evidence pass.
- Verify Claude Code and Gemini execution against supported real CLI releases
  and user entitlements, retaining the common cancellation/recovery contract.

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
