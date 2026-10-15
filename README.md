# Patchfleet

Your coding agents. One local Rust desktop command center.

Patchfleet 0.9 runs its interface and host in Rust. egui/eframe renders the native
window through one Glow backend; the installed application contains no WebView,
HTML/JavaScript bundle or Node server. Local canonical state remains useful
without Cloud. Accessibility support is enabled.

The native interface supports Turkish and English. Change the language in the
sidebar; the owner Host saves it locally for the next launch. Source messages,
work instructions, code and custom user content retain their original language.

Cloud sign-in is optional. Configure the Cloud server address, then use the
system browser to sign in and approve this installation. Account and subscription
status appear in the desktop; sign-out keeps Local projects and downloaded ADRs.
Local remains free and works without an account. See [account state](docs/state/desktop-account.md).

Create stable projects, link multiple Git worktrees, keep project messages and
owner task/team sources in local Project Memory. Create sourced ADR drafts in
Decisions, explicitly accept/reject them and replace an accepted decision while
preserving its evidence and history. Queue and control supported Codex work, configure twelve
agent packs and six bounded team templates, answer owner gates, and inspect
provider status. Claude Code and Gemini CLI remain observation-only. Provider
CLIs and their own model/account/runtime requirements are installed separately.

Rendering reacts to input and state changes. Owned run signals are checked
quickly during active work; idle provider probes are limited to once a minute.
Cloud receives only allowlisted operational data by default. Optional paid
shared memory publishes only explicitly previewed and confirmed accepted ADRs.
Source bodies, paths, prompts, output and credentials remain local by default.

## Build and run

Install Rust 1.99+ and platform native compiler/window prerequisites, then:

```sh
cargo run --locked -p patchfleet-desktop
cargo build --locked --release -p patchfleet-desktop
```

The production binary is `target/release/patchfleet-desktop` (Windows `.exe`).
Node/npm are not required to build or run the current desktop. Historical
`app/`, `lib/`, `bin/` and Node tests remain event/protocol parity references.
`legacy:*` scripts target that historical host, and npm publication is retired.

## Package and verify

```sh
cargo install cargo-packager --version 0.11.8 --locked
cargo packager --config Packager.toml --formats app,dmg
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Build the executable before packaging. Local packages are under `dist/native`.
Native CI targets macOS ARM64/Intel, Windows x64 and Linux x64/ARM64. Local Mac
build/runtime evidence and remaining signing/platform checks are recorded in
[native state](docs/state/native-rust-desktop.md).

Read [installation and recovery](docs/install.md), [architecture](docs/architecture.md)
and [release checks](docs/release.md). [Project Memory](docs/state/project-memory.md)
supports durable sources, message revisions, sourced ADR lifecycle and scoped
keyword reads. The optional subscription Cloud memory service supports explicit
publication and exact cross-device reads; see the [memory protocol](docs/memory-protocol.md).
Explicit Cloud semantic search and free Local ranked keyword retrieval are implemented; see [search state](docs/state/project-memory-search.md). Agent memory tools and automatic drafts remain later slices. Only
owner acceptance makes an ADR active; agent context injection is not enabled yet.
Local memory remains free. Cloud service deployment is a separate release step.

## Product boundaries

- Local execution and append-only events are canonical.
- Cloud cannot execute arbitrary shell commands.
- Declarative packs cannot load code or widen permissions.
- Lost or uncertain work is never silently restarted.
- No private work data or credentials upload by default.

## License

Patchfleet is [Apache License 2.0](LICENSE).
Report vulnerabilities through the [security policy](SECURITY.md).
