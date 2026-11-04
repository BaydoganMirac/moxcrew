# ADR 0041: Managed project job directories

Status: Accepted for owner-authorized Task0040 implementation

Date: 2026-10-19

## Decision

Use a private per-job output directory beneath the local data directory for
general-purpose Project work. Reuse the existing supported Codex app-server
controller and durable work/project facts. The local Host resolves project and
job identity and authorizes only the exact expected canonical directory.

Provide a separate managed-directory validation path for prepare and start.
Ordinary registered/manual coding work keeps Git validation. A user-provided
boolean, source instruction or arbitrary path cannot select the managed path.
Symlink aliases and escapes must fail closed, including result reads/exports.

Unix result operations use pinned directory handles. Windows result access is
explicitly unsupported until equivalent race-resistant protection is implemented
and natively verified. The original implementation retained a partial new
destination on write failure; [ADR0051](0051-atomic-owner-export-publication.md)
supersedes that Unix export detail with private staging and atomic no-replace
publication. Cleanup never removes the requested public pathname.

Native Create uses declarative, localized ability recipes and does not claim
their required renderers, models or connectors exist. Actual files can be
inspected and explicitly exported with overwrite protection. Their presence
does not independently establish the quality or completion of an ability.

V8 owner requested/receipted events retain the exact input and semantic result
for project job enqueue, result export and work retry. V1 queue/run facts and
V3 project binding retain their previous schemas and meanings. The project
revision gate runs inside the queue transaction; rejection does not invent a new
V1 reason code. Existing readers without V8 support must fail closed rather
than editing or skipping the new facts. No Cloud version is negotiated.

## Preserved boundaries

Native Rust/egui, one local writer, existing provider access, explicit queue/
start and project decision scope remain. No API key, new provider engine,
Node/WebView, hosted execution, generic plugin runtime or new Cloud fields are
introduced. Presentation/video tool feasibility remains separate evidence.

## References

- [Task0040](../plans/0040-project-create-studio.md)
- [ADR0051](0051-atomic-owner-export-publication.md)
- [1.0.0 plan](../plans/0039-version-one-release.md)
