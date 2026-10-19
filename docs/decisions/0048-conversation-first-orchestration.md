# ADR0048: Conversation-first orchestration

Status: Accepted by owner request; Task0050

Date: 2026-10-19

The owner reports that Chat still behaves like a configuration form and requires
working through conversation. The entry surface is therefore one message box,
one Send action and an immediately visible local provider choice. Recipes,
work modes, rosters and workflow configuration remain optional details.

The first explicit Send can create a local Project for that request. Continue
only after the exact creation command's applied receipt and the authoritative
created Project are present; never infer ownership from a name, first list item
or a late unrelated receipt. Merely opening Chat creates no Project or work.

Use existing real provider planning and clarification operations. Display actual
owner requests and provider-returned questions/plan state, without invented model
answers. Follow-up messages retain the original goal and bounded prior context;
unsupported input limits are visible rather than silently truncating history.
Provider selection is separate from installation, saved authentication and
execution capability. One bounded diagnostic read may run on entry, while
explicit refresh remains available after failure. Do not source shell startup
files or bypass capability checks to make a provider appear ready.

Sending a message authorizes the clearly indicated selected-provider planning
call. It does not start an executing crew or approve an external operation.
Existing Prepare and explicit Start controls belong to the relevant chat turn
and retain exact Project, plan, mission, revision and receipt checks. No natural-
language keyword silently grants execution authority.

This refines ADR0047's chat hierarchy and ADR0044's conversational orchestration.
Existing local event meanings, provider credentials and the Cloud boundary stay
unchanged. Task0050 records the implementation and verification limits.
