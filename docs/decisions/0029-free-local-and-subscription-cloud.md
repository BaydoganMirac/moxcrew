# ADR 0029 Free Local and subscription Cloud

Status: Accepted product boundary; runtime transition not implemented

Date: 2026-10-15

Owner direction: Local is free; Cloud services require a subscription.

## Decision

Patchfleet Local remains useful and free without an account or Cloud
subscription. Local projects, agent teams and local memory belong to this
boundary. Planned decision management, retrieval, scoped agent access and
source-backed ADR drafts remain Local capabilities as they are implemented.
This record does not claim those planned slices already exist.

Patchfleet Cloud is an optional subscription service. The paid value is hosted
storage/search, explicitly enabled project-memory sharing, cross-device sync,
remote supervision, notifications and managed recovery as those capabilities
are implemented. A permanent Cloud Free tier is outside the new product model.
Whether to offer a trial remains undecided.

The desktop does not require a local PostgreSQL server for subscribing. Cloud
PostgreSQL with pgvector is the proposed shared-memory backend; its schema,
embedding model, sync authority and protocol need a separate implementation
decision. Local storage remains a separate technical choice.

The free application does not make a user's coding provider or model access
free. Provider costs remain separate. Hosted compute/storage allowances must
be explicit; no unlimited hosted inference is promised.

## Data and subscription behavior

- Sharing is enabled explicitly per project. Payment alone never enables
  source, prompt, transcript, credential or path uploads. The default Cloud
  allowlist remains unchanged.
- Shared ADR publication needs a separately versioned memory contract;
  billing and ordinary pairing do not authorize it.
- Cloud cancellation or expiry never disables Local, erases downloaded owner
  data or stops locally owned agent work. Cloud export, expiry and retention
  behavior must be specified before the new paid service launches.
- Owner acceptance remains required for an authoritative ADR. Search relevance
  and subscription status cannot promote a draft.

## Delivery boundary

The current Cloud Free/Solo implementation remains unchanged. This owner
decision supersedes its commercial packaging, not existing protocol semantics,
customer data or runtime entitlements. No existing-customer transition is
assumed. Price, billing unit, quotas, trial and expiry policy need a bounded
follow-up card before enforcement or publication.

No billing configuration, migration, deployment or data transfer is authorized
by recording this product boundary. Keep Local independent of Cloud throughout.

## References

- [Product](../product.md)
- [Local-first boundary](0001-local-first-public-runtime-private-cloud.md)
- [Memory contract](0021-project-scoped-local-decision-memory.md)
- [Memory implementation plan](../plans/0022-project-memory-and-decision-service.md)
