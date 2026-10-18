# AGENTS.md

This file is the entry point for AI coding assistants working in Moxcrew.
Read it before changing code or documentation.

## What this repository owns

Moxcrew is the public local product:

- provider detection and adapters;
- local agent and run observation;
- local work intake and control;
- canonical local events and derived projections;
- the native desktop console;
- the public host-to-cloud protocol.

Moxcrew must remain useful without Moxcrew Cloud. The private Cloud repo
implements an optional remote control plane; it does not own canonical run
state and does not execute local work.

## Required reading order

1. [docs/product.md](docs/product.md)
2. [docs/architecture.md](docs/architecture.md)
3. [docs/protocol.md](docs/protocol.md) for any host/cloud change
4. [docs/roadmap.md](docs/roadmap.md)
5. The applicable project skill under [.agents/skills/](.agents/skills/)
6. The relevant file under [docs/plans/](docs/plans/)
7. The relevant file under [docs/state/](docs/state/)
8. Applicable records under [docs/decisions/](docs/decisions/)

## Tool entry points

- Codex and compatible tools read AGENTS.md directly.
- Claude Code reads [CLAUDE.md](CLAUDE.md), which points back here.
- Gemini reads [GEMINI.md](GEMINI.md), which points back here.

Do not create tool-specific copies of project rules.

## Project skills

Read a matching skill completely before acting:

- [provider-adapters](.agents/skills/provider-adapters/SKILL.md) — provider
  discovery, observation, capabilities, and conformance.
- [local-runtime-state](.agents/skills/local-runtime-state/SKILL.md) — events,
  projections, persistence, replay, commands, and receipts.
- [cloud-boundary](.agents/skills/cloud-boundary/SKILL.md) — pairing, sync,
  sanitization, intents, authorization, and retention.
- [team-task-contract](.agents/skills/team-task-contract/SKILL.md) — team roles,
  delegation, file ownership, review, and handoff.

## Non-negotiable rules

1. Local state is canonical. Cloud state is a sanitized, rebuildable
   projection.
2. No arbitrary remote shell command exists. Remote actions are versioned,
   allowlisted intents that the local host validates.
3. Do not upload source code, diffs, prompts, transcripts, tool output,
   environment variables, credentials, tokens, or absolute paths by default.
4. Claude Code, Codex, and Gemini CLI integrations implement the same small
   domain contract. Provider-specific parsing and process behavior stay inside
   provider adapters.
5. Add abstractions only after a second real implementation proves the need.
6. Prefer the Rust standard library before adding a core dependency. Node remains only a legacy parity reference, not a desktop build or runtime requirement.
7. The local deployable is a native Rust/egui application for macOS, Windows
   and Linux. Keep one canonical writer; do not add a browser engine, Node
   sidecar or separate execution authority. ADR 0024 supersedes the Tauri
   interface after the owner's explicit direction and lightweight requirement.
8. Durable changes are written as append-only events by one local writer;
   user-facing state is a rebuildable projection.
9. Every command is idempotent, attributable, time-bounded, and produces a
   receipt.
10. Never place machine-specific absolute paths, credentials, or private Cloud
    implementation details in this repository.
11. Product runtime changes start only from an approved task card. Cloud
    infrastructure remains deferred until local control is useful and its own
    implementation task is approved.

## Change discipline

- Make surgical changes tied to one plan milestone.
- Add tests at the behavior boundary, especially adapter conformance,
  persistence recovery, redaction, idempotency, and authorization.
- Update the relevant state file at the end of a work session.
- Record a non-obvious product, protocol, security, or architecture choice as
  an ADR before relying on it.
- Workers may commit locally. They do not push, merge, deploy, publish
  packages, or apply migrations without explicit owner direction.

## Definition of done

A task is not done until:

- its acceptance criteria pass;
- no forbidden data crosses the Cloud boundary;
- affected docs and state are current;
- the staged diff contains only intended files;
- a local commit is ready for owner review.
