# Patchfleet

Your coding agents. One local desktop command center.

Patchfleet 0.3 runs a Rust host inside a Tauri desktop application with a React
interface. The installed application does not need a Node server. It keeps
canonical state on the developer machine and remains useful without Cloud.

The console provides provider observation, durable work intake, supported
Codex start/cancel, workspace registration, twelve declarative agent packs and
six bounded team templates. Teams advance in the background with concurrency,
retry, time-budget and owner decision gates. Claude Code and Gemini CLI remain
observation-only; provider CLIs are installed separately.

Patchfleet Cloud is an optional private SaaS receiving sanitized operational
projections and returning typed, expiring intents. Local work text, paths,
prompts, output and provider credentials stay local by default.

## Desktop development

Install [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), Rust,
and Node 22.18+ or Node 24 for frontend tooling, then run:

```sh
npm ci
npm run desktop:dev
```

Build the native application and platform installers:

```sh
npm run desktop:build
```

Build artifacts are under `target/release/bundle/`. The desktop CI matrix builds
macOS ARM64/Intel, Windows x64, and Linux x64/ARM64 on their platform runners.
Local macOS development packages are verified; other native platform runs and
public signing remain release checks. See [runtime state](docs/state/rust-desktop.md).

## Validation

```sh
npm test
npm run build
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

The old `app/`, `lib/`, `bin/` and Node tests remain development references for
event/protocol compatibility. `legacy:*` commands explicitly target that old
host; it is not a dependency or sidecar of the desktop. npm package publication
has been retired for this desktop source package.

Read [installation and recovery](docs/install.md), the
[documentation map](docs/README.md), and [architecture](docs/architecture.md).
Project-scoped shared memory and automatic sourced ADR drafts are the next
feature phase, with explicit owner acceptance as the binding decision boundary.

## Product boundaries

- Local execution and its append-only events are canonical.
- Cloud cannot execute arbitrary shell commands.
- Packs contain data and instructions; they cannot load code or widen permissions.
- A lost or uncertain run is marked honestly, never silently restarted.
- No source, prompts, transcripts, worktree paths or secrets upload by default.

## License

Patchfleet is [Apache License 2.0](LICENSE).
Report vulnerabilities through the [security policy](SECURITY.md).
