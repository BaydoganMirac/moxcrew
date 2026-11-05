# Task0096: Verified branded campaign assets

Status: Complete; locally verified

Updated: 2026-11-05

Authorization: the owner asked to continue closing missing product features after
the desktop interface overhaul. This card adds a deterministic local path for
the I4 cover/card and bounded I5 brand-consistency abilities without a model,
provider account, network request or publication action.

## Problem

Project can reach the local image engine for free-form generation and editing,
but it cannot turn exact approved copy, Project colours and optional frozen
visuals into a consistent set of common campaign formats. I4 and I5 therefore
have catalog and engine access without a checked, owner-visible local artifact
pack.

## Scope

1. Add one typed `images.campaign` production operation using the existing
   isolated Python/Chrome worker with page scripts, network and downloads off.
2. Accept exact bounded title, subtitle, eyebrow and call-to-action copy; a
   five-colour brand; one to four unique known formats; and optional frozen
   PNG/JPEG hero and logo resources.
3. Produce an editable SVG and exact-size PNG for every requested format plus
   canonical content, a source/output-hashed manifest and a concise review
   summary.
4. Add a native Project production form that stages this exact operation in
   Chat. The form never executes production directly and remains usable in the
   supported narrow desktop viewport in English and Turkish light/dark themes.
5. Route I4 and the bounded deterministic part of I5 through real Python/Chrome
   readiness rather than claiming that a configured generative image engine is
   sufficient.

## Acceptance

- Only `square`, `landscape`, `portrait` and `thumbnail` formats are accepted;
  duplicates, unknown keys, unbounded copy, invalid colours and low-contrast
  palettes fail closed.
- Optional visuals are exact current job inputs, limited PNG/JPEG files and are
  embedded as data URIs; generated SVG/HTML contains no external reference,
  script or caller-provided markup.
- Every PNG has the declared dimensions and valid PNG structure. Every emitted
  asset plus canonical content and summary hash is recomputed into the manifest
  before the job completes; the returned receipt separately identifies the
  final manifest hash.
- The output count remains inside the existing production limit, with at most
  four SVG/PNG pairs plus content, manifest and summary.
- Native actions stage the typed request in Chat and do not issue a Host
  production command. Geometry/action tests cover the smallest supported
  viewport and both locales/themes.
- Focused worker, routing, readiness, schema, UI and screenshot evidence pass;
  affected documentation and state are current; the local commit contains only
  intended files.

## Limits

This operation proves exact copy, palette, source identity, dimensions and file
integrity. It does not prove copy quality, visual taste, trademark rights,
conversion performance or arbitrary third-party brand-guideline compliance.
Those remain human review. No provider/model call, account mutation, network
request, upload, publish, push, tag or release is authorized by this card.
