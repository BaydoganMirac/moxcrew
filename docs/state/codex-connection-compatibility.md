# Codex connection compatibility

Updated: 2026-10-25

Status: Adapter implemented and verified; installed application update pending

Plan: [Task0055](../plans/0055-codex-connection-compatibility.md).

## Failure and repair

The owner reported `CODEX_PROTOCOL_MALFORMED`. The installed Codex CLI was
0.163.0, executable discovery succeeded and existing authentication was present.
Reinstalling or signing in again would not address this failure.

A real read-only reproduction found that `thread/read` with `includeTurns:true`
returned more than the adapter's 256 KiB JSON-line limit for a long conversation.
Observation needs status metadata, so the adapter now requests one newest turn
through `thread/turns/list` with `itemsView:notLoaded`. Thread identity, creation
time and native status come from the already validated `thread/list` entry.
The verified version gate is 0.163.0+, rather than a claim about the first release
that introduced pagination. Older supported versions keep their existing path.

The change preserves process cleanup, timeout/output ceilings, normalized
session fields, explicit execution and provider-owned authentication. It adds no
dependency, configuration, credential handling, data migration or Cloud field.
Oversized and malformed responses still fail closed.

The supported interface is documented in the
[official app-server reference](https://learn.chatgpt.com/docs/app-server).
Native payloads, conversation content, identifiers and machine paths were not
copied into this repository or verification output.

## Verification

- Provider conformance: **10 passed**, including newest-turn pagination, empty,
  running and terminal states, malformed pages/status, oversize rejection and
  secret-field exclusion. The fixture makes any full-history read exceed the
  same limit, so the regression reproduces the original defect.
- Provider extraction: **5 passed**. Provider memory: **7 passed**.
- Native desktop: **229 passed** with all features. Loopback tests required
  running outside the restricted sandbox; the initial sandbox-only run failed
  two loopback-dependent tests, which passed on the complete rerun.
- Strict workspace/all-target/all-feature Clippy: passed with `-D warnings`.
- Real adapter under a minimal GUI-style PATH: Codex 0.163.0 **available**, **20**
  normalized recent sessions, authentication **present**, control supported.
- Direct read-only protocol comparison: the previous full-history response
  exceeded 256 KiB; each latest-turn metadata response was under 600 bytes.

Verification used version/login-status and observation requests only. No model
turn, new login, credential-file inspection or Project mutation was requested.

## Application handoff

Production release build and replacement of the existing local macOS App are
pending. No new DMG or archive is needed. The existing retained fallback App
matches the previously installed executable and remains available for rollback.

Windows/Linux live-provider runs and a paid model turn are outside this repair's
verification. No push or publication has been performed.
