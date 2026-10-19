# Selected-provider execution

Status: Implemented, integrated and independently reviewed; live-provider evidence remains separate

Updated: 2026-10-19

[Task0044](../plans/0044-productive-project-crews.md) adds real Claude Code and
Gemini CLI headless execution alongside the existing Codex app-server controller.
The Host owns V13 persistence, authorization, queueing and recovery; adapters only
own provider processes, configuration and normalized signals.

## Implemented

- Controller prepare/start/cancel/drain dispatches by the exact selected provider.
  New headless surfaces require stable Claude Code >=2.1.170 or Gemini CLI >=0.43.0.
  A version-supported executable does not establish authentication, account access,
  or tool availability. Gemini hook setup is independent of owned headless control.
- Prepare validates the directory and creates an ephemeral private configuration
  and session UUID. It does not send the goal or start inference. Start dispatches
  the prompt through stdin, using the supported `--session-id` flag. V13 process
  attempts use `providerTurnId=process:<runId>` because headless CLIs expose no
  native turn ID. Native init must confirm the expected session; success requires
  an explicit successful result followed by a successful process exit.
- File read/edit tools and the exact run-scoped stdio MCP server are configured.
  Shell, browser, independent agents, discovered skills, hooks and extra MCP
  servers are excluded. Typed production operations run through Host tools.
  Claude uses `dontAsk` with directory-scoped file permission rules; Gemini uses
  `auto_edit` with a fixed builtin allowlist. Neither uses permission bypass/YOLO.
- Claude keeps existing subscription authentication: `--bare` is deliberately not
  used because the verified version disables OAuth/keychain authentication there.
  Settings sources are excluded and context discovery disabled using explicit
  settings. Credentials are never inspected or copied by Moxcrew.
- Gemini uses its supported per-process system-settings path, retaining the CLI's
  authentication. If an existing system-settings override or administrator settings
  file is detected, the adapter refuses with `PROVIDER_CONFIG_CONFLICT`; it does
  not replace administrator policy. No user configuration file is modified.
- Cancellation and shutdown revoke MCP access immediately and reap only owned
  process trees. Terminal native results revoke access before the next Host poll.
  A bounded output reader rejects malformed, oversized, foreign-session, incomplete
  or trailing output. Native bodies/errors are not normalized into events.
  Typed local production workers reuse this bounded process lifecycle. The
  deadline includes stdin, stdout, stderr and process exit; cancellation or a
  dropped caller still reaps the owned tree, including descendants retaining
  stdout after their parent exits. Windows uses the existing suspended process
  and Job object path; only Unix behavior has been executed locally.
- `generate_structured(provider, scratch, prompt, schema, active)` is a separate
  tool-free worker. Codex retains the existing extraction path; Claude uses native
  JSON schema output; Gemini receives the schema in stdin and returns exact JSON.
  All results must be JSON objects; the caller validates its specific schema.
  Total timeout is 60 seconds, prompt <=256 KiB, transcript <=1 MiB, result <=128 KiB.
  Running work has a 16 MiB transcript cap and two-hour adapter ceiling; shorter
  mission limits remain enforced by the Host.
- `drain_usage()` emits terminal provider-reported input/output counts and Claude's
  reported USD cost in integer millionths when supplied. Missing fields remain
  absent, including Gemini cost. No prices or entitlement assumptions are made.
  Codex uses `thread/tokenUsage/updated.tokenUsage.total`, matching both owned
  thread and turn. Each fresh owned thread receives one turn; only its latest
  cumulative total is emitted at terminal status. Foreign and unknown counts are
  ignored and repeated drain calls do not duplicate measurements.

## Tool readiness

Connection diagnostics report `configured_unverified` for new CLI adapters.
Claude's native init must report the scoped MCP server connected. Gemini's
headless init does not expose a tool inventory, so configuration alone is not a
verified discovery claim. Every actual call remains checked against the live
Host binding and Project scope. Existing Codex pre-turn MCP inventory/context
checks are unchanged.

## Verification

- Installed CLI read-only checks: Claude Code 2.1.170 and Gemini CLI 0.43.0
  `--version` / `--help`; no login, credential read or live inference.
  Installed Codex protocol JSON schema was generated into a disposable directory
  to verify the actual usage notification fields without any model call.
- Native deterministic peers exercise prepare without inference, actual start and
  file output, duplicate/foreign start rejection, native usage, unknown usage,
  malformed/foreign/oversized/incomplete/permission-denied/failed streams, exact
  cancellation, child cleanup, terminal lease revocation, tool-free generation,
  input limits and unsupported versions.
- Existing Codex observation/control, memory and extraction suites retained.
  Isolated adapter verification: 17 provider unit tests and 9 conformance tests
  pass; extraction (5) and memory (7) suites pass. Strict all-target core Clippy
  passes. Native process cleanup tests ran with process inspection permitted.
  Two additional typed-worker tests cover stream/exit validation, inherited
  stdout, deadline, explicit cancellation and dropped-caller cleanup. Final
  integrated totals and independent review belong to the task state.
- Integrated Host regression runs both native CLI peers through a managed
  artifact revision, an actual failed process and owner retry. It verifies V13
  and provider identity, original frozen bytes/manifest despite changed parent
  output, conversation visibility, tamper rejection and durable retry receipts
  after reopening the Host. This focused check passes with the local MCP
  listener permitted; it uses no real account or model call.

## Limits

Live model/account access and native Windows/Linux execution are not verified.
Provider permission systems differ from Codex's workspace sandbox. Organization
policies may deny otherwise configured tools. Claude config isolation excludes
user `apiKeyHelper` settings; supported existing OAuth/keychain/environment access
remains provider-owned. Gemini has no non-inference authentication-status API in
this integration. Closed headless sessions are never silently resumed, and a lost
Host does not reattach to an old process. These are capability boundaries, not a
claim that a recipe or connected provider guarantees a valid artifact.

## Sources

- [Claude headless CLI](https://code.claude.com/docs/en/headless)
- [Claude CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Gemini headless interface](https://geminicli.com/docs/cli/headless/)
- [Gemini configuration](https://geminicli.com/docs/reference/configuration/)
- [Gemini MCP configuration](https://geminicli.com/docs/tools/mcp-server/)
- [Codex app-server](https://learn.chatgpt.com/docs/app-server)

## Task0050: desktop NVM discovery

- Confirmed the installed Codex npm shim was available in a terminal's NVM PATH
  but absent from the adapter's previous standard desktop search directories.
  Calling that shim by absolute path with a minimal desktop PATH also failed
  because `/usr/bin/env node` could not find its runtime.
- On Unix, existing PATH and standard install-directory precedence is retained.
  Discovery now additionally checks an absolute `NVM_DIR`, or the standard
  `HOME/.nvm`, under `versions/node`. It accepts stable numeric `vMAJOR.MINOR.PATCH`
  directory names, requires an executable sibling Node and provider file, and
  tries versions newest first. Enumeration is limited to 128 entries; an
  oversized or unreadable directory is ignored instead of selecting a random
  partial result. Shell profiles and NVM scripts are never evaluated.
- For an absolute executable provider with a verified executable sibling Node,
  only the child's PATH gains that containing directory. The parent's environment,
  selected executable and arguments remain unchanged. Injected `Programs`,
  authentication checks, version gates, deadlines and process cleanup retain
  their existing contracts. There is no executable picker or automatic install.
- Deterministic discovery tests cover numeric precedence, PATH precedence,
  malformed/missing/oversized directories, non-executable candidates, child-only
  PATH changes and an npm-style shim requiring Node under a minimal PATH.
- A disposable executable linked to the current compiled core was launched with
  `PATH=/usr/bin:/bin:/usr/sbin:/sbin` and no `NVM_DIR`. `Programs::default` found
  Codex in the standard NVM directory; its real bounded connection check returned
  Codex **0.163.0**, `authentication: present` and `CODEX_CREDENTIALS_PRESENT`.
  Only `--version` and `login status` were invoked; other providers were disabled
  for this check. Output retained only normalized fields, never account details.
  An earlier standalone version probe reached its five-second deadline once;
  subsequent checks succeeded. The deadline remains unchanged.
- This evidence establishes local discovery and reported credential presence,
  not model access, quota, account entitlement or successful execution. No live
  inference, login, installation or credential-file inspection occurred. Native
  Windows/Linux behavior and other Node version managers remain unverified.
