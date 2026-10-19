# Explicit Project app connections

Status: Adapter implemented; Host integration and review in progress

Updated: 2026-10-19

[Task0044](../plans/0044-productive-project-crews.md) supports three concrete
operations through the native Host. The owner supplies a token for each selected
Project and app; no browser login, credential discovery, token refresh or account
mutation happens during setup or preview.

- `google_drive / drive_import`: an explicit file ID retrieves bounded metadata
  and the blob contents. Metadata must confirm the ID and a size up to 25 MiB;
  returned bytes must match that size. The file name becomes a safe local basename.
  Google Docs, Sheets and Slides require export and return
  `PROJECT_CONNECTION_GOOGLE_EXPORT_REQUIRED`; blob download is not presented as
  Google-native document support.
- `notion / notion_page`: a selected parent page ID, title and plain text become
  a page with typed paragraph blocks. Unicode text is split within the documented
  rich-text bound. The response exposes only a checked page ID.
- `google_calendar / calendar_event`: a calendar ID, summary, description and
  offset-bearing start/end timestamps create a single event. End must follow
  start. A bounded IANA-shaped zone identifier is validated by the remote API.
  No attendees are accepted. Event IDs derive deterministically from the local
  Project and prepared action; arbitrary native request fields are rejected.

Credentials are confined to a private Project file (0700 directory / 0600 file
on Unix; existing protected-storage ACL handling on Windows). Tokens never enter
canonical events, metadata or native error output. Configuration and removal use
private command digests and receipts; replay cannot restore a removed token or
switch to different arguments. Receipt storage is bounded and fails explicitly
when full. Each app has an independent `connectionRevision` so the Host can reject
a prepared action after its account configuration changes.

`connected` and `configured` indicate that an owner-supplied token exists;
`authentication: unverified` prevents setup from claiming live account access.
Preparation performs exact argument validation without network or mutation.
The Host owns the concrete preview, local approval, durable dispatch fact,
revision checks, resource publication and crash recovery.

Execution uses fixed HTTPS hosts, encoded path segments, no redirects and a
10-second total deadline, including both Drive requests. JSON bodies are bounded
to 256 KiB and files to 25 MiB. Remote error text and arbitrary response fields
are discarded. A lost, oversized, malformed or 5xx write response produces an
unknown outcome. No automatic retry occurs, including after a possible Notion
write; the Host retains that uncertainty in its action state.

Verification uses synthetic tokens and loopback HTTP fixtures for all three
operations, redaction, Project isolation, credential revision/replay, private-file
permissions and symlink denial, argument allowlists, malformed/oversized output,
redirect refusal, timeouts and uncertain writes. All seven focused tests pass;
loopback binding ran with sandbox restrictions lifted for the fixtures. No real account or service call
has been performed. Live credentials, scopes, remote permissions and native
Windows/Linux protected storage remain unverified.

Sources:

- [Drive files.get and blob/export distinction](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/get)
- [Calendar events.insert and explicit event IDs](https://developers.google.com/workspace/calendar/api/v3/reference/events/insert)
- [Notion page creation and paragraph blocks](https://developers.notion.com/guides/data-apis/working-with-page-content)
- [Notion request limits and uncertain write responses](https://developers.notion.com/reference/request-limits)

Notion requests pin the documented `Notion-Version: 2026-03-11` contract.
