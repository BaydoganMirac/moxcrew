# ADR 0026 Native desktop language preferences

Status: Accepted

Date: 2026-10-14

The owner explicitly added language support to the approved native memory slice.
The first languages are Turkish and English, with Turkish as the default.

Compile UI translations into the native application using a small explicit table;
no runtime language server, browser bundle or translation model is needed. Labels,
help, empty/error states and built-in catalog display copy are translated. Owner
messages, instructions, custom pack data, identifiers and provider brand names keep
their original content. English keys are the fallback for unrecognized custom copy.

The in-app language selector calls the serialized owner Host. It saves only a
versioned language preference in protected `desktop-preferences.json`. This display
configuration cannot change project scope, source evidence, accepted decisions,
provider permissions or the Cloud protocol. The native UI never writes it directly.

Use V3 owner audit events for the new setting command, preserving V2 owner actions
and strict V1 records. Existing audit request/receipt identity, expiry and retry
semantics apply. A crash with an unfinished setting request remains visibly unknown
rather than replaying a side effect. A damaged preference file falls back to Turkish
without losing canonical data; only an explicit language selection replaces it.

Expose current language and a safe error flag in local snapshots. Cloud's existing
allowlists ignore this configuration. Native localization and preferences tests
cover switching, restart, invalid language and unchanged owner source text.
