# Codex connection compatibility

Updated: 2026-10-25

Status: Complete; installed App updated and live Codex observation verified

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
- Production App: built without `ui-smoke`, all three fixture environment markers
  absent, licenses retained, ad-hoc signature passes strict verification. The
  signed App was launched with a minimal PATH and disposable empty Project data;
  it reports Codex 0.163.0 available with 20 sessions. Only this test process was
  terminated and its temporary data removed.
- Independent source review of `586268b`: no actionable blocker; bounds,
  correlation, redaction, cleanup and legacy compatibility retained.
- Formatting, whitespace and affected Markdown links pass.
- Installed App after owner-approved restart: fresh observation at
  `2026-10-25T20:56:59.744Z`, Codex 0.163.0 **available**, **20** sessions,
  no provider error. The updated installed process remains running.

Verification used version/login-status and observation requests only. No model
turn, new login, credential-file inspection or Project mutation was requested.

## Application handoff

The first replacement guard found the installed App running and stopped before
modifying it. The owner explicitly authorized restarting it. The running process
was stopped, the signed App replaced through a staged bundle swap with rollback,
and the updated App reopened through Launch Services. Its strict signature and
executable hash match the verified package. No data migration, login reset or
manual change to Project files was performed.

The installed version remains 0.12.0, with build `20261025.205253`. Final signed
executable SHA-256:
`833211af8e27de2d20b416da6d7bdc40aa168c8235146abb6111c16b6ef99703`.
This also carries Task0054's previously uninstalled composition improvements.
The retained fallback App matches the previous installed executable and remains
available for rollback. No new DMG or archive was created. Release intermediates
were removed after verification (`cargo clean --release`: 1,928 files / 708.2 MiB
logical); the remaining target directory is approximately 1.3 GiB. The temporary
staging App was removed after installation.

Runtime commit: `586268b`. Local verification records are under
`../output/branding/moxcrew/codex-connection-qa/` relative to the repository root.
The package manifest distinguishes the release/linker signature hash
from the final App executable hash after bundle signing.

Windows/Linux live-provider runs and a paid model turn are outside this repair's
verification. No push or publication has been performed.
