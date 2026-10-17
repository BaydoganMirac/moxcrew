# Feature state index

| Feature | Status | Last updated | State |
|---|---|---|---|
| V0 foundation | Done | 2026-07-15 | [v0-foundation.md](v0-foundation.md) |
| V0 Codex observation | Done | 2026-07-15 | [v0-codex-observation.md](v0-codex-observation.md) |
| V0 provider observation contract | Done | 2026-07-16 | [v0-provider-observation-contract.md](v0-provider-observation-contract.md) |
| V0 provider lifecycle contract | Done | 2026-07-16 | [v0-provider-lifecycle-contract.md](v0-provider-lifecycle-contract.md) |
| V0 Claude Code observation | Done | 2026-07-16 | [v0-claude-observation.md](v0-claude-observation.md) |
| V0 Gemini CLI observation | Done | 2026-07-16 | [v0-gemini-observation.md](v0-gemini-observation.md) |
| V0 provider production integration | Done | 2026-07-16 | [v0-provider-integration.md](v0-provider-integration.md) |
| V0 Gemini lifecycle ingress | Done | 2026-07-16 | [v0-gemini-lifecycle-ingress.md](v0-gemini-lifecycle-ingress.md) |
| V0 local work control | Done | 2026-07-16 | [v0-local-work-control.md](v0-local-work-control.md) |
| V0 paired Cloud control | Done | 2026-07-16 | [v0-paired-cloud-control.md](v0-paired-cloud-control.md) |
| V0 closed alpha readiness | Done | 2026-07-17 | [v0-closed-alpha-readiness.md](v0-closed-alpha-readiness.md) |
| V0 publishable Local beta | Done | 2026-07-17 | [v0-local-beta.md](v0-local-beta.md) |
| External beta activation | In progress | 2026-07-17 | [external-beta-activation.md](external-beta-activation.md) |
| Agent packs and teams | Done | 2026-07-20 | [agent-teams.md](agent-teams.md) |
| Project/source memory and automatic ADR drafts | Local ADRs, Cloud publication, retrieval, agent tools and optional extraction implemented | 2026-10-15 | [project-memory.md](project-memory.md) |
| Rust and Tauri desktop | Superseded interface; core retained | 2026-10-14 | [rust-desktop.md](rust-desktop.md) |
| Native Rust desktop interface | Implemented; platform release checks pending | 2026-10-15 | [native-rust-desktop.md](native-rust-desktop.md) |
| Scoped agent memory | Complete; independent review and macOS development package verified | 2026-10-15 | [agent-memory.md](agent-memory.md) |
| Automatic sourced ADR drafts | Complete; reviewed and macOS development package verified | 2026-10-15 | [automatic-drafts.md](automatic-drafts.md) |
| Product studio interface | Complete; independently reviewed and macOS package verified | 2026-10-15 | [product-studio-interface.md](product-studio-interface.md) |
| Dobbly native interface | Detailed plan prepared; implementation not started | 2026-10-17 | [dobbly-brand-interface.md](dobbly-brand-interface.md) |
| Optional desktop account | Implemented; compatible hosted service/provider release checks pending | 2026-10-15 | [desktop-account.md](desktop-account.md) |

Phase 1 local observation, Phase 2 durable local work control, and Phase 3
paired Cloud visibility with remote cancel are complete.
Phase 4 packaged that loop and made the private production foundation
deploy-ready without expanding its remote command surface. Hosted release
activation remains an explicit owner action. The Local beta adds safe doctor
diagnostics and a Node 22/24 macOS, Windows, and Linux release matrix without
changing the Cloud protocol.

External beta release preparation is locally verified. Publication, hosted
release, live receipt proof, and moderated-user evidence remain explicit
release gates rather than being recorded as completed work.

The 0.4 native Rust/egui desktop supersedes the 0.3 Tauri interface while retaining
the Rust core and data directory. Its state file records current native artifacts
and remaining platform release checks. Version 0.5 adds stable project/source memory
and Turkish/English display. Version 0.6 adds Local sourced ADR acceptance;
version 0.7 adds optional explicit paid Cloud publication and exact reader copies.
Version0.8 adds measured Cloud vector retrieval and free Local keyword search;
version0.9 adds optional native account sign-in. version0.10 adds scoped Local agent tools. Version0.11 adds consented automatic sourced ADR drafts.

State files are resumable handoffs. Update the relevant file whenever a work
session changes implementation status, blockers, decisions, or next steps.
