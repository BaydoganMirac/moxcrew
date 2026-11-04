# Task0064: Owner-visible system check

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked whether the rebuilt desktop actually works and
asked to implement the remaining useful features. This slice adds an honest,
non-destructive system check. Windows remains deferred to a Windows PC.

## Problem

Provider connection diagnostics, production-tool readiness, the image engine
and Project app connections already have bounded local reads, but their results
live on separate screens. The owner cannot answer “is this Project ready?” from
one place, and a generic green state could easily overclaim live model access or
output quality.

## Scope

1. Turn the existing Providers page into a combined System check surface while
   preserving detailed provider observations below it.
2. Run the existing `provider_connections_read` and, for one selected Project,
   `project_studio_read` sequentially from one explicit owner action.
3. Summarize local engine, usable provider connections, production tools, image
   engine and configured app connections with honest ready/setup/unchecked
   states and direct routes to existing setup surfaces.
4. Keep the check read-only: no install, sign-in, provider turn, remote app
   action, artifact creation or automatic repair.
5. Cover command sequencing, Project isolation, status classification,
   navigation and minimum-window behavior.

## Non-goals

- No claim that saved credentials prove model entitlement, quota or output
  quality.
- No replacement for per-ability start preflight or Host authorization.
- No new canonical event, Cloud field or durable readiness cache.
- No Windows packaging or runtime verification.

## Acceptance

- One explicit action refreshes connection diagnostics and the selected
  Project's existing Studio readiness without overlapping commands.
- Results never cross Project boundaries and unchecked evidence is never shown
  as ready.
- Missing setup routes to the existing Connections, Project tools, Images or
  Apps experience; the system check itself does not mutate setup.
- Focused and full workspace tests, strict Clippy, formatting and diff checks
  pass, with native visual evidence for the finished surface.
