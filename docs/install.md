# Install and operate Patchfleet

Status: beta release contract

Updated: 2026-07-20

## Requirements

- macOS, Windows, or Linux;
- Node.js 22 or newer and npm (Node 24 is also tested);
- supported provider CLIs installed separately.

The owner supplies a reviewed Patchfleet npm tarball. Installation and package
publication are separate owner release actions:

    npm install --global ./patchfleet-0.2.0.tgz

Operate the local host with:

    patchfleet workspace add .
    patchfleet workspace list
    patchfleet doctor
    patchfleet start
    patchfleet status
    patchfleet stop
    patchfleet recover

Inspect or install declarative local agent packs with:

    patchfleet agent-pack list
    patchfleet agent-pack show pack:orchestrator
    patchfleet agent-pack install ./pack.json
    patchfleet agent-pack remove pack:custom

The browser console forms and controls bounded multi-agent teams from the
registered projects and installed packs. Pack manifests are versioned JSON;
they contain no executable hooks and cannot expand provider permissions.

Run `patchfleet workspace add .` from each Git worktree you want to use. The
command validates and stores its canonical local path; the console then lets
you select the project by name. Remove an entry with the opaque ID printed by
`workspace list`:

    patchfleet workspace remove workspace:...

Workspace paths remain in the owner-only local event log and projection. They
are not included in Patchfleet Cloud payloads.

The UI binds only to `http://127.0.0.1:3000`. Override the port with
`PATCHFLEET_PORT`. Runtime state and logs live under `~/.patchfleet` by default;
override only for testing with `PATCHFLEET_DATA_DIR`.

On macOS and Linux, Patchfleet enforces mode `0700` on its data directory and
`0600` on runtime metadata and logs. On Windows, the default directory stays
inside the current user's profile and inherits that profile's ACL. Do not point
`PATCHFLEET_DATA_DIR` at a shared folder.

Run `patchfleet doctor` whenever installation, startup, project selection, or
Cloud pairing is unclear. It reports only bounded health information; it does
not print credentials, event contents, prompts, paths, or provider output. An
empty first install produces actionable warnings rather than a failure.

`recover` requires the host to be stopped. It validates the Cloud connection
file and rebuilds derived observation, work, and workspace projections from the
durable event log. It never invents a receipt or retries a provider side
effect.

## Upgrade

Stop the host, install the reviewed replacement tarball, run recovery, and
start again:

    patchfleet stop
    npm install --global ./patchfleet-NEW_VERSION.tgz
    patchfleet recover
    patchfleet start

Do not delete `~/.patchfleet` during an upgrade. Copy that directory while the
host is stopped for a local backup. Restore it only to the same or a compatible
newer Patchfleet version, then run `patchfleet recover`.

## Uninstall

Stop the host before removing the package:

    patchfleet stop
    npm uninstall --global patchfleet

The owner data directory is deliberately retained so an uninstall cannot erase
acknowledged local work. After confirming that no backup or reinstall is
needed, remove `~/.patchfleet` using the operating system's normal file manager.

## Provider compatibility

Use the provider cards in the console or `patchfleet doctor` to distinguish an
unavailable CLI from an observation-only integration. Codex is the only beta
provider with proven start and cancel control. Claude Code and Gemini CLI are
observation-only until their supported interfaces expose an equally bounded
lifecycle control surface.

## Gemini lifecycle extension

The package includes `extensions/patchfleet-gemini`. Link that directory with
Gemini CLI, restart Gemini, and use Gemini's native extension status/uninstall
commands. Its sanitized inbox uses the same Patchfleet user-data directory.
