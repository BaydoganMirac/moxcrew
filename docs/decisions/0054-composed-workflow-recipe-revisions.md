# ADR0054: Version the ten composed workflow recipes

Status: Accepted for owner-authorized Task0100

Date: 2026-11-05

## Decision

Moxcrew will represent each approved J1–J10 composition as one bundled recipe
using ADR0052's frozen earlier-member dependencies. Existing launch, podcast,
study, travel, content and research recipes advance to revision2 and become the
current J1, J2, J3, J4, J7 and J9 graphs. J6, J8 and J10 receive new stable
built-in IDs. The legacy data recipe remains unchanged and data-briefing remains
the verified J5 graph.

Each composition has at most six members. The final reviewer depends on every
producer. Dependency edges only move checked files; they never grant provider,
tool, network, account, app-action, publication or decision-acceptance authority.
Prepare and Start remain separate owner actions.

## Compatibility

Changing a built-in body requires a higher recipe revision. Previously persisted
revision1 recipe/schedule facts retain their exact frozen body and remain valid
under the same built-in ID; they are not silently rewritten into the new graph.
New selections use the current revision2 body and stale expected revisions fail
through the existing command contract. Saved owner recipes continue to freeze
their exact members, settings and dependency matrix.

The catalog gains only the three compositions that had no existing natural
entry. It does not add a second workflow language, new durable event meaning,
Cloud fields, automatic migration or another execution service.
