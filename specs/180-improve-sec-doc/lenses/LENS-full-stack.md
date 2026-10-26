# Lens — Full Stack Developer

[Pack index](../README.md) · [Requirements](../03-product-spec.md) · [Assessment](../02-current-implementation.md) · [Plan](../13-implementation-plan.md) · [Tests](../12-e2e-test-matrix.md) · [Research](../14-research-and-benchmark.md)

**First principle:** Preserve one access intent across every transport and every persistence boundary.

## Code grounding

Extend clearance.rs services/handlers, ingest_admission.rs, document_admission.rs, performFileUpload and current clearance API/hooks. Existing domain allows/allows_relabel remain policy authority.

## Key decisions

Introduce a typed admission access envelope, capabilities/read-model endpoints, advisory draft preview and strict versioned mutation. Never implement upload labeling as upload then PATCH.

## DRY and SOLID

Keep normalization, capability composition, transaction mutation and transport parsing in focused modules. UI fields consume data/callbacks; adapters depend on shared types; domain rules do not depend on HTTP or React.

## Edge focus

EC-180-07/08/09/17/21/27/28/29/32/34/36/40. MCP finalization, worker upsert and duplicate reingestion are first-class contract callers.

## Acceptance

T-180-05/07/08/09/12/17/21/24/28/29/32/34/40. Use shared contract fixtures and real DB/auth negative tests; a UI mock is not policy evidence.

## Tradeoff

Workspace epoch serialization is intentionally coarse. Measure contention and document any redesign; don't replace correctness with a TTL or cached browser capability.

## Implementation ownership

Lead WP-02/03 and integration across WP-01/04/05/07. Keep generated OpenAPI/client schemas in the same reviewable change as new API shapes.

The lens is a review perspective applied within this pack, not a claim of a separate independent expert review. Resolve conflicting recommendations against [the policy laws](../01-first-principles.md), then record a concrete requirement/test change.
