# Install and operate Patchfleet

Status: closed-alpha release contract

Updated: 2026-07-17

## Requirements

- macOS or Windows;
- Node.js 20.9 or newer and npm;
- supported provider CLIs installed separately.

The owner supplies a reviewed Patchfleet npm tarball. Installation and package
publication are separate owner release actions:

    npm install --global ./patchfleet-0.1.0.tgz

Operate the local host with:

    patchfleet start
    patchfleet status
    patchfleet stop
    patchfleet recover

The UI binds only to `http://127.0.0.1:3000`. Override the port with
`PATCHFLEET_PORT`. Runtime state and logs live under `~/.patchfleet` by default;
override only for testing with `PATCHFLEET_DATA_DIR`.

On macOS, Patchfleet enforces mode `0700` on its data directory and `0600` on
runtime metadata and logs. On Windows, the default directory stays inside the
current user's profile and inherits that profile's ACL. Do not point
`PATCHFLEET_DATA_DIR` at a shared folder.

`recover` requires the host to be stopped. It validates the Cloud connection
file and rebuilds derived observation/work projections from the durable event
log. It never invents a receipt or retries a provider side effect.

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

## Gemini lifecycle extension

The package includes `extensions/patchfleet-gemini`. Link that directory with
Gemini CLI, restart Gemini, and use Gemini's native extension status/uninstall
commands. Its sanitized inbox uses the same Patchfleet user-data directory.
