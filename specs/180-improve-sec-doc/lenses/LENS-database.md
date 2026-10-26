# Lens — Database Expert

[Pack index](../README.md) · [Requirements](../03-product-spec.md) · [Assessment](../02-current-implementation.md) · [Plan](../13-implementation-plan.md) · [Tests](../12-e2e-test-matrix.md) · [Research](../14-research-and-benchmark.md)

**First principle:** Make every label change atomic, scoped, auditable and race-safe.

## Code grounding

Migration 220 stores document labels both as join rows and UUID arrays, with copied columns on seven child tables. Current relabel reads old state before begin and lacks version preconditions.

## Key decisions

Add label_version and scoped mutation receipts. Treat join rows as compartment authority and arrays as a transactional projection. Strengthen workspace/kind integrity and policy-only vocabulary references.

## Concurrency proof

Lock workspace policy first, documents in sorted order, then tags/children. Refresh subject grants inside that boundary. All policy-affecting writers participate. Child writes lock parent FOR SHARE; relabel uses FOR UPDATE.

## Operational tradeoff

Synchronous propagation is acceptable only within measured bounds. Above the aggregate descendant cap, reject unchanged rather than acknowledge a partially protected corpus. Future async relabel needs quarantine semantics.

## Edge focus

EC-180-09/11/12/16/27/28/30/35/37/38/39/40. Include RLS-owner exceptions, connection reuse, audit partition errors, rollback/WAL and migration restart.

## Acceptance

T-180-09/11/12/16/20/27/28/30/35/37/38/39/40 under the actual application role, with deterministic race barriers and EXPLAIN plans.

## Implementation ownership

Lead WP-01, review every WP-03 child writer, WP-06 filter/count predicate and WP-07 race/performance gate. Keep schema expansion reversible without removing security state.

The lens is a review perspective applied within this pack, not a claim of a separate independent expert review. Resolve conflicting recommendations against [the policy laws](../01-first-principles.md), then record a concrete requirement/test change.
