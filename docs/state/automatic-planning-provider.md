# Automatic planning connection

Updated: 2026-10-20

Status: Complete; independently reviewed, native captures and macOS development package verified.

Plan: [Task0051](../plans/0051-automatic-planning-provider.md).
Decision: [ADR0049](../decisions/0049-automatic-planning-connection.md).

## Product flow

The default composer takes the request and Send, without requiring a provider
choice. Mox uses existing diagnostics to resolve a supported usable local
connection in stable Codex/Claude Code/Gemini CLI order. The real planner then
interprets the goal and proposes abilities and crew members. This does not claim
a measured model-quality ranking, bundled model access or offline inference.

Provider selection and Automatic remain optional secondary controls. An explicit
unavailable override cannot silently use another account. If no usable connection
exists, keep the message and offer Connections. Existing installation, version,
authentication and capability checks remain authoritative.

Resolve and freeze the actual provider before the first Send/Project creation.
Changes to diagnostics, preferences or late receipts cannot retarget the frozen
request or trigger cross-account inference retries. Original goals, labelled
clarification context, scoped files and exact Project receipts retain their
contracts. Prepare and Start remain explicit owner actions.

## Verification

- Complete all-feature desktop suite: **187 passed, 0 failed, 0 ignored**.
  Its 79 focused Create/conversation checks also passed separately. Strict
  workspace/all-target/all-feature Clippy, workspace formatting and diff checks
  pass. Core code is unchanged; its Task0050 integration suite was not repeated.
- Readiness tests cover only-Claude, only-Gemini, multiple/no usable providers,
  unsupported/old Codex and missing authentication. Explicit unavailable or
  unknown overrides fail closed rather than falling back. Real Send payloads
  carry concrete provider IDs, including advanced planning and assignments.
- Pending Project creation stores the actual provider separately from the
  preference. Changed diagnostics never redirect the command: the original
  provider is retained if usable, otherwise the Project/message survive for an
  explicit resend. Active request status also retains its frozen provider.
- Minimum-window TR/EN light/dark interactions prove the default composer has no
  provider picker and Send is reachable without configuration. Keyboard selection
  of an unavailable manual provider under More disables Send; a real pointer
  selection of Automatic restores it without changing the draft or issuing work.
- Independent source and actual native image review found no remaining issue in
  `adf3c54` plus integration `92af1cd`. Existing scoped files, context, receipts
  and explicit Prepare/Start contracts remain intact.
- **44 actual native captures** cover the five conversation states in TR/EN,
  light/dark and normal/minimum windows, plus only-Claude/Gemini TR dark windows
  at both sizes. Provider metadata and plans are illustrative fixtures without
  provider probes or inference. The sibling output index
  `branding/moxcrew/automatic-connection-qa/native/qa.json` records source commit,
  binary/image hashes and dimensions. Fixture binary SHA-256:
  `60741a005b54c7553deb1df0da9d1686e6ddfce996d8b5766586d3084290db80`.

## Development package

`dist/native/automatic-connection/Moxcrew_0.12.0_automatic-connection_aarch64.dmg`

Version remains 0.12.0. The macOS ARM64 development package is ad-hoc signed,
without public notarization. Strict signature verification, license resources,
absence of smoke/gallery code, isolated normal launch/close, writer-lock release
and DMG integrity verification pass. Evidence:
`dist/native/automatic-connection/qa.json`. Runtime source: `92af1cd`.

- Executable: 16,637,776 bytes; SHA-256
  `bd7d6c66d24a2a7150a286381366a8207bbbc48f749c114bcaf6f288f5326c23`.
- DMG: 8,565,147 bytes; SHA-256
  `b2937ab5a175512af8defbf0c2b9fd8336739f346fad1d0b08f0b089cf256f2b`.

No core, event schema, Cloud boundary or dependency change. No live inference,
login, installation, push or deployment. Native Windows/Linux execution,
live model/account behavior and public signing remain unverified.
