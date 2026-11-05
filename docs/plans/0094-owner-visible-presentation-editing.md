# Task0094: Owner-visible presentation editing

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: The owner asked to close remaining non-Windows feature gaps and
make Mox excellent. This card exposes the already verified P2 presentation
revision engine in the native desktop app without a live provider/model call,
dependency installation, publication or cleanup.

## Problem

`presentations.revise` already replaces one exact slide and verifies that every
non-target slide remains byte-identical, but the owner cannot prepare that typed
operation from Project tools. P2 therefore exists in the worker and catalog but
is missing a first-class desktop entry point.

## Scope

1. Add a native **Edit presentation slide** form for one exact Project PPTX,
   exact one-based slide number, layout, owner text, optional speaker notes and
   optional real render verification.
2. Support bounded cover, statement and content edits without exposing arbitrary
   XML, script or hidden mutation.
3. Stage the exact typed request in Chat; preserve explicit plan, review and
   Start steps and never execute directly from the form.
4. Localize the complete form in Turkish and keep its primary action reachable
   at 800×560 in Turkish/English and light/dark themes.
5. Retain a real Turkish dark native capture and point to the existing checked
   revision/preservation evidence.

## Non-goals

- No automatic rewrite, semantic copy generation, slide reordering or deletion.
- No arbitrary PowerPoint XML, animations, SmartArt or third-party layout repair.
- No live model, cloud upload, publication, account action or cleanup.
- No Microsoft PowerPoint or Windows behavior claim.

## Acceptance

- Incomplete/invalid layouts cannot stage a request; exact source, slide, layout
  and every owner string appear in the typed operation.
- Form actions emit no direct Host production command and keep Start explicit.
- The action remains reachable at 800×560 in both languages and themes.
- Existing core revision tests still prove exact non-target slide preservation.
- Focused tests, strict Clippy, formatting and diff checks pass.

## Outcome

Project tools now exposes **Edit presentation slide** as a complete native form.
It stages one bounded `presentations.revise` operation for an exact Project PPTX
and slide, with cover, statement and content layouts, owner-authored text,
optional notes and optional render verification. The form never executes the
worker directly and keeps plan review and Start under the owner's control.

The full form is localized in Turkish, its action remains reachable at the
minimum supported window size in both themes and languages, and a real Turkish
dark native capture is retained. The existing verified worker and preservation
tests remain the execution source of truth; this task adds no second revision
path.
