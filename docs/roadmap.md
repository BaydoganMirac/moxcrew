# Patchfleet public roadmap

Status: active

Updated: 2026-07-17

This roadmap describes public technical direction, not dates or commercial
commitments.

## Now

- Keep the completed Phase 1 observation, Phase 2 local control, and Phase 3
  paired Cloud control loops stable.
- Validate pairing, sanitized visibility, remote cancel, receipts, revocation,
  restart behavior, and offline local use with real users.
- Keep the local/Cloud trust contract and contributor rules reviewable.
- Validate the local workspace registry and project-selection flow in clean
  package installs; absolute paths remain local-only.

## Next

- Validate the completed macOS, Windows, and Linux package, diagnostics,
  upgrade, uninstall, and recovery paths with external beta users.
- Validate the private self-service Cloud Free release candidate and its
  explicit migration through owner-operated deployment and two-user testing.
- Generalize execution control to Claude Code or Gemini only after a provider
  proves an equally safe supported lifecycle surface.
- Introduce declarative local agent packs only after external activation is
  stable and two repeated workflows require reusable setup. An executable
  plugin SDK needs two real integrations plus a separate ADR and threat model.

## Later

- Add team capabilities when real multi-user demand defines them.
- Consider a desktop shell, provider SDK, or self-hosted Cloud only when the
  web-first product proves a concrete need.
- Add multi-agent orchestration only after single-agent ownership,
  cancellation, receipts, concurrency limits, and failure isolation are
  proven.

Patchfleet remains useful without Patchfleet Cloud throughout every stage.
