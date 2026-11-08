# ADR 0055: Use one locale registry and catalog fallback

Status: Accepted

Date: 2026-11-06

## Context

Moxcrew currently supports reviewed Turkish and English desktop copy. The
translation lookup is centralized, but the supported-code allowlist and
selector metadata are duplicated. Repeating language conditionals in product
screens would not scale and would risk translating owner-controlled content.

## Decision

The native product uses one compile-time locale registry as the authority for
supported codes, native labels and fallback behavior. UI screens request copy
from one catalog lookup. English source strings remain the stable fallback;
reviewed locale catalogs override those strings. Dynamic owner, model,
provider and protocol text is never implicitly translated. Placeholder values
are inserted only after the template lookup.

Preference reads may fall back to Turkish after malformed local state, while a
preference mutation with an unsupported code is rejected. Future locales are
added to the registry and a reviewed catalog, not to individual screens.

## Consequences

- Locale support has one discoverable extension point and consistent failure
  behavior.
- Shipping a locale still requires complete human-reviewed copy and layout
  evidence; registry support alone is not a translation-quality claim.
- Full ICU plural and grammatical selection remain a later explicit decision
  if product copy needs them.
