<picture>
  <source media="(prefers-color-scheme: dark)" srcset="design/moxcrew/logos/moxcrew-reversed.svg">
  <img src="design/moxcrew/logos/moxcrew-primary.svg" alt="Moxcrew — Mox + crew" width="320">
</picture>

# Moxcrew

One idea. A whole crew.

Moxcrew helps people turn goals into outcomes with AI agents, organized in
projects with persistent context and decisions. Its intended uses include
research, writing, analysis, planning, personal work and software development.
Project is the main product concept; a separate Workspace hierarchy is not
required. See the [product direction](docs/decisions/0038-general-purpose-projects.md).

Talk to Mox in a Project, inspect and edit its plan, then explicitly start a
crew. Supported local Codex, Claude Code and Gemini CLI providers can be assigned
per member. Crews exchange scoped messages and checked file deliveries; owner
direction can replace affected work while preserving existing outputs. Reviewers
accept checked results or request bounded corrections. Saved recipes and local
schedules reuse this workflow. See [workflow state](docs/state/workflow-controls.md)
and [provider execution](docs/state/provider-execution.md).

Output Studio keeps files, versions, imported Project resources, tool setup and
app actions beside Chat. Typed local tools create editable PPTX/DOCX/XLSX files,
clean and analyze CSV, resize images, edit short videos and clean audio. Office/data
tools require Python 3.9+; media operations require owner-installed FFmpeg/FFprobe.
PDF text extraction additionally uses Poppler. Native Office previews show extracted
structure; media previews show metadata and a thumbnail. Original files remain
exportable. These bounded operations do not imply arbitrary format compatibility,
full document rendering, media playback or forty verified end-to-end abilities.
See [Output Studio state](docs/state/output-studio.md).

Each Project has Chat, Crew and Outputs views inside the same native window.
Chat keeps the request and conversation central; Crew leads with the current plan
and its next action. Outputs gives files and previews their own space. Detailed
provider, budget and setup controls open when needed, without covering the whole
application with a second control panel.

Google Drive blob import, Notion page creation and Google Calendar event creation
use private, manually configured access tokens. Each action has a concrete preview
and a separate explicit execution command; uncertain remote writes are never retried
automatically. See [app connections](docs/state/app-connections.md).

The approved [1.0.0 release target](docs/plans/0039-version-one-release.md)
covers eight capability areas, forty abilities and ten composed project workflows,
including presentations and video editing. See the
[release state](docs/state/version-one-release.md) for implementation evidence;
1.0.0 is a future target, not the current application version.

Moxcrew 0.12 runs its interface and host in Rust. egui/eframe renders the native
window through one Glow backend; the installed application contains no WebView,
HTML/JavaScript bundle or Node server. Local canonical state remains useful
without Cloud. Accessibility support is enabled.

The Moxcrew interface groups Chat, projects, tasks and teams beside project memory and
decisions. Project cards open their own context; creating a task from a project
selects its local folder while preserving any existing unsent draft. Bundled UI
fonts, Mox mascot and the vector Moxcrew mark work offline. Both light and dark
themes follow the system preference. The brand source is in [design/moxcrew](design/moxcrew/README.txt).
The refreshed studio centers Mox and eight capability cards above a focused
composer, with ink/cobalt surfaces and an adaptive file panel. See the
[native studio evidence](docs/state/futuristic-native-studio.md).

The native interface supports Turkish and English. Change the language in the
sidebar; the owner Host saves it locally for the next launch. Source messages,
work instructions, code and custom user content retain their original language.

The application opens Chat with the Project conversation and one composer.
It suggests a task type from explicit goal terms, with manual selection and an
inspectable catalog. Chat, Projects, Teams and Tasks are the primary navigation;
other tools remain available under More. Connections separates installation,
saved sign-in and supported task control for the three local CLIs. Setup guides
open by deliberate action. Saved credentials do not establish model access.
Mox moves during real active work; reduced motion is saved locally.

Cloud sign-in is optional. Configure the Cloud server address, then use the
system browser to sign in and approve this installation. Account and subscription
status appear in the desktop; sign-out keeps Local projects and downloaded ADRs.
Local remains free and works without an account. See [account state](docs/state/desktop-account.md).

Create stable projects, link multiple Git worktrees, keep project messages and
owner task/team sources in local Project Memory. Create sourced ADR drafts in
Decisions, explicitly accept/reject them and replace an accepted decision while
preserving its evidence and history. Queue and control supported local provider work, configure twelve
agent packs and six bounded team templates, answer owner gates, and inspect
provider status. Provider
CLIs and their own model/account/runtime requirements are installed separately.

Rendering reacts to input and state changes. Owned run signals are checked
quickly during active work; idle provider probes are limited to once a minute.
Cloud receives only allowlisted operational data by default. Optional paid
shared memory publishes only explicitly previewed and confirmed accepted ADRs.
Source bodies, paths, prompts, output and credentials remain local by default.

Moxcrew = **Mox + crew**: Mox is the mascot; crew is the cooperating agent team.
The brand domain is `moxcrew.com`; service deployment is a separate step. Existing installation, data,
command and protocol identifiers retain `patchfleet` for compatibility.

The public repository is [BaydoganMirac/moxcrew](https://github.com/BaydoganMirac/moxcrew).

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
and [release checks](docs/release.md). Create/retry/export commands write V8 local
events; chat-send and motion preferences add V9 audit facts. Project crews add
V10 local mission and communication facts. Productive crews add V11 workflow,
V12 resource/artifact/action and V13 multi-provider execution/usage facts. Older builds need
an unchanged pre-upgrade backup for rollback.
[Project Memory](docs/state/project-memory.md)
supports durable sources, message revisions, sourced ADR lifecycle and scoped
keyword reads. The optional subscription Cloud memory service supports explicit
publication and exact cross-device reads; see the [memory protocol](docs/memory-protocol.md).
Explicit Cloud semantic search and free Local ranked keyword retrieval are implemented; see [search state](docs/state/project-memory-search.md). Scoped Local agent tools and optional consented automatic ADR drafts are implemented; see [draft state](docs/state/automatic-drafts.md). Only
owner acceptance makes an ADR active; active agents read it through scoped memory tools.
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
