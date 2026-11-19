# Windows file delivery and desktop preview

The owner approved Windows file delivery as a prerequisite for publishing the
x64 desktop preview. The release uses the current native Moxcrew installer
identity and a new version tag.

## Acceptance

- A project can list, preview and export a real file on Windows, then deliver
  selected outputs in a ZIP with matching bytes and hashes.
- Result reads stay within the project. An existing destination survives a
  collision. Failed exports leave no partial file at the requested path.
- Storage retains the owner and SYSTEM permissions across reopen and recovers
  a partially written final event. Existing projects keep their identity.
- The packaged executable opens a disposable profile and closes normally.
  The release workflow validates the exact tag, installer set and checksums.

The core builder owns Windows result access and archive integration. The
coordinator owns the desktop entry points, native release gates and integration.
Review the stable changes and exercise the native installer before publication.
