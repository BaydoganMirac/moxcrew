# Task0055: Repair Codex connection diagnostics and observation

Status: Complete; installed App updated and live Codex observation verified

Updated: 2026-10-25

Authorization: owner cannot connect Codex and requests a fix; exact reported
diagnostic is CODEX_PROTOCOL_MALFORMED.

## Scope and acceptance

Trace the actual installed Codex 0.163.0 machine-readable response against the
adapter's bounded parser. Preserve privacy, timeouts, output bounds, session
identity, provider-controlled authentication, frozen execution and explicit Start.
Use only read-only version/login status and app-server observation calls for live
diagnosis; no model request or credential-file inspection.

Correct the smallest proven compatibility issue in the provider adapter and
cover it with deterministic provider conformance tests. Keep malformed/oversized
untrusted data rejected; do not solve this by unbounded transcript reads. Update
UI diagnostics only if they misclassify this proven error. Verify real installed
CLI observation after the fix and update the existing application as needed
without accumulating DMG/archive versions or changing Project data.

No provider reinstall, login reset, API-key provisioning, new runtime authority,
Cloud change or automatic model execution is part of this fix. Record exact test
results, real read-only evidence and any installed-app update limits in state.

## Implementation evidence

Real Codex 0.163.0 reproduced the failure: a full-history `thread/read` response
exceeded the existing 256 KiB JSON-line ceiling. The adapter now uses one
metadata-only `thread/turns/list` page for verified 0.163.0+ versions, preserving
the legacy path for older supported versions. Output/time bounds remain intact.

Ten provider conformance tests, five extraction tests, seven provider-memory
tests and 229 desktop tests pass. Strict workspace/all-target/all-feature Clippy
passes. The updated adapter reports the real Codex provider available, observes
20 recent sessions and confirms existing authentication without a model call.
The owner authorized restarting the running App; the signed production build is
installed and running with a fresh available Codex observation and no error.

Current handoff: [Codex connection state](../state/codex-connection-compatibility.md).
