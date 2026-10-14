# Patchfleet public roadmap

Status: active

Updated: 2026-10-14

This roadmap describes public technical direction, not dates or commercial
commitments.

## Now

- Prepare project-scoped agents/teams and shared local decision memory under
  [Task 0022](plans/0022-project-memory-and-decision-service.md): use the
  [accepted Rust host](decisions/0023-rust-tauri-desktop-runtime.md), then
  establish stable Project identity and owner source capture before local
  retrieval, scoped agent access, and automatic ADR drafts with explicit
  acceptance.
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
- Validate the self-service Cloud Free release and the full Local-to-Cloud team
  journey with moderated external users.
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
