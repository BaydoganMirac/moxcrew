# Task 0014: Product-ready local activation

Status: Completed

Owner approval: 2026-07-17

## Goal

Turn the proven local control loop into a clear first-use product experience
without changing the canonical event model, provider contract, or Cloud
protocol.

## Scope

- Present one obvious activation path: refresh providers, queue work, start it,
  then monitor or cancel it.
- Show local, Codex-control, and optional Cloud readiness near the top of the
  console.
- Keep user-correctable work-form failures inside the console through safe,
  allowlisted feedback instead of rendering a raw API response.
- Show durable command outcomes in plain language while retaining the stable
  reason code for support.
- Improve responsive hierarchy, keyboard focus, contrast, and empty states
  using the existing Next.js and CSS stack.

## Out of scope

- New commands, provider capabilities, protocol fields, dependencies, billing,
  telemetry, or remote enqueue.
- Uploading local work text, paths, provider output, or other forbidden data.
- Deploying, publishing, pushing, or changing live Cloud data.

## Acceptance criteria

1. A new user can identify the next required action from the first viewport.
2. Queue, start, cancel, and receipt remain separate durable steps and expose
   only controls supported by the current projection.
3. User-correctable form errors and terminal command receipts redirect back to
   an accessible, allowlisted console banner; arbitrary query text is never
   rendered.
4. The console remains usable at desktop and narrow mobile widths with visible
   focus states and no horizontal overflow.
5. Existing browser-boundary, persistence, idempotency, privacy, Cloud, build,
   and package checks continue to pass.

## Verification

- Focused local-shell tests for redirects, safe messages, and primary flow.
- Full public test suite.
- Production build and clean package lifecycle smoke.
- Browser review at desktop and mobile widths against the running local host.

## Result

- Added one product hierarchy for privacy, readiness, first-run guidance, work
  intake, current work, receipts, Cloud pairing, and provider detail.
- Redirected safe work-form and command outcomes back to an accessible
  allowlisted banner; unknown and prototype query keys render no banner.
- Verified a real owner flow on the paired production host:
  `WORK_ENQUEUED` -> `WORK_STARTED` -> `RUN_CANCELLED`, ending with the work item
  and run interrupted and the Cloud projection synced.
- Full public suite passed 130/130, followed by the production build, clean
  package lifecycle smoke, desktop review, and 390-pixel mobile review with no
  horizontal overflow.
- Live Cloud-authored cancel remains the Stage 0 exit gate because the browser
  used for this verification did not have an authenticated owner session.
