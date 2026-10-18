# ADR 0037: Moxcrew display identity with installation compatibility

Status: Accepted; Moxcrew name, moxcrew.com and implementation selected by owner

Date: 2026-10-18

## Context

After the earlier Dobbly planning, the owner purchased moxcrew.com and
authorized the coordinated Moxcrew desktop/Cloud implementation. The selected
concept-4 mascot is named Mox. The naming structure is Mox + crew: Mox is
the character; crew expresses the cooperating team and community spirit. The existing product is a native
Rust/egui application with a separate optional Cloud console. Existing installs
and canonical data use Patchfleet technical identifiers.

## Decision

- Moxcrew is the visible product identity; Mox names the mascot; Moxcrew Cloud labels the optional
  service. Use the selected cobalt/orange asymmetric-eyed helper and the
  approved Manrope-based design contract.
- Maintain shared versioned brand inputs and checked-in product-specific
  exports. Neither executable nor Cloud build requires the sibling repository,
  external font/CDN or a brand exporter at runtime.
- Keep bundle/app identity `com.patchfleet.desktop`, Rust/Node package names,
  executable names, protected data paths, environment variables, event/action
  identifiers and public protocols compatible. A visible rename does not
  authorize migration or deletion of owner data.
- Expose light and dark palettes using the OS preference, without changing the
  strict persisted language-preference schema. Manual persistent theme settings
  are a separate bounded change.
- Preserve Local's account-optional operation, actual provider capabilities,
  explicit consent and receipt-based completion. Cloud continues to render
  only authorized data; matching visual style does not mean matching data access.
- Broaden the clarity and tone of presentation without claiming unsupported
  non-coding workflows or changing the implemented product contract.

## Consequences

Technical and visible names temporarily differ deliberately. Repository links,
download destinations, support identities, merchant/legal operator names and
configured domains remain accurate until their owners change them explicitly.
Text/asset checks must distinguish obsolete visible branding from intentional
compatibility identifiers; a global text replacement is not acceptable.

The presentation part of [Task 0036](../plans/0036-product-studio-interface.md)
is superseded only as [Task 0037](../plans/0037-dobbly-brand-interface.md) is
implemented. The native architecture, trust contracts and previous verification
history remain valid. This decision authorizes the presentation boundary. Completion and release
remain separate facts recorded by the implementation state.
