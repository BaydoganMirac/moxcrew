# ADR 0018: Support Node 22 and 24 for the public beta

Status: Accepted

Date: 2026-07-17

## Context

The installable host still declares Node 20 even though that release line is
end-of-life. Patchfleet needs one conservative minimum that works on current
developer machines and one forward-compatibility check without multiplying the
release matrix.

## Decision

Require Node 22 or newer. Run the full macOS, Windows, and Linux package path on
Node 22, plus one Linux compatibility job on Node 24. Keep the local and Cloud
applications on the same minimum runtime until a hosting constraint requires a
documented exception.

## Consequences

- Node 20 exits the supported surface.
- Node 22 remains the compatibility floor for the beta.
- Node 24 regressions are caught without doubling every operating-system job.
- A future minimum-runtime change requires another release decision.
