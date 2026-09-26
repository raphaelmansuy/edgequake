# WHY sequence the work around correctness gates

A new adapter would currently inherit incomplete contracts. Fix those first, then separate composition, complete durable relational commands, and prove one provider substitution at a time. Work packages below are **planned**, not completed. Dependencies are release gates, not suggested shortcuts.

This document owns package scope. [feasibility assessment](feasibility-and-releases.md) assesses feasibility and release boundaries; [implementation guide](step-by-step-guide.md) breaks the packages into J01-J24 with prerequisites and child PRs. [persistence recipes](persistence-and-provider-recipes.md) supplies concrete recipes; [E2E specification](../validation/e2e-test-specification.md) and [definition of done](../validation/definition-of-done.md) define executable proof and completion.

```ascii
W0 Baseline --> W1 Safety contracts --> W2 Composition
                                          |
                                          v
                               W3 Relational commands
                                          |
                                          v
                               W4 Durable projections
                                          |
                                          v
                               W5 Unified vector path
                                          |
                                          v
                               W6 Bounded graph path
                                          |
                                          v
                               W7 Qdrant --> W8 Neo4j
                                                |
                                                v
                                      W9 SQLite + closure
```

## PROVIDER-ACCESS-W0 — Pin evidence and make certification honest

**Owner:** storage/test infrastructure. **Dependencies:** none. **Findings:** F14 and all baselines.

Extend [storage_backend_contract.rs](../../../edgequake/crates/edgequake-storage/tests/storage_backend_contract.rs) and its support modules into shared provider fixtures; retain existing assertions. Add proposed `tests/support/access_contract/` helpers and a manifest distinguishing unit fixtures, real provider runs and skipped tests. Build a reviewed SQL/driver dependency inventory from the F02 footprint. Record P0 behavior, operation counts and performance using [algorithms and budgets](../design/algorithms-and-budgets.md).

Add a required CI job with explicit database provisioning: missing credentials/service is **failure**, not successful early return. Optional local tests may still skip, but emit machine-readable skip status and cannot satisfy certification. Separate the existing LLM provider compatibility tests from storage-provider certification.

**Exit:** baseline fixtures and version/digest manifest retained; T20 detects unavailable fixtures; every F01–F14 has an owning package/test. **Rollback:** test-only; no storage changes.

## PROVIDER-ACCESS-W1 — Close current safety holes before adding providers

**Owner:** storage/query integration. **Dependency:** W0. **Findings:** F05–F09/F13.

Change [vector.rs](../../../edgequake/crates/edgequake-storage/src/traits/vector.rs), [graph_mutate_ops.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph_mutate_ops.rs), [graph_read_ops.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph_read_ops.rs), [error.rs](../../../edgequake/crates/edgequake-storage/src/error.rs), [typed_read.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/typed_read.rs), and [storage_impl.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs).

1. Add regression fixtures with two tenants, same names, multiple documents/modalities/models and non-ready chunks. Call the **typed** filtered branch directly to prove every predicate survives routing.
2. Carry all filters into the typed query and enforce the serving fence on its early-return path. Preserve index-compatible predicates; make backend failures typed errors rather than empty success. Translate these into explicit API failure/degradation behavior.
3. Require implementations for safety-critical filtered/mutation methods; update current memory/PG implementations and thin test doubles explicitly. Optional capabilities return UnsupportedCapability.
4. Require exact triple-edge deletion and actual MergeSources/Replace semantics. Do not remove the current correct adapter overrides.
5. Introduce normalized error classification using SQLSTATE/status; test uniqueness, deadlock, serialization retry, rate limit and unknown outcome.

**Exit:** current-adapter regression cases of T01/T02/T07/T08/T12/T13 pass on memory and P0, with error-mapping tests. Transaction/crash/stale-worker cases T03-T06 require W3/W4 and are not W1 prerequisites. **Compatibility:** preserve public empty-list semantics through explicit API translation; scope-null legacy data goes through an offline ownership census/repair, not permissive search. **Rollback:** old routing only if it also satisfies the new safety tests; never re-enable ignored filters to restore availability.

## PROVIDER-ACCESS-W2 — Introduce independent provider composition

**Owner:** runtime/storage. **Dependency:** W1. **Findings:** F01/F12.

Add proposed `edgequake-storage-contracts` leaf crate; move driver-free IDs/errors/ports and re-export temporarily. Add proposed `state/data_access_factory.rs` and `state/data_access_config.rs`. Refactor [state/postgres.rs](../../../edgequake/crates/edgequake-api/src/state/postgres.rs), [storage_runtime.rs](../../../edgequake/crates/edgequake-api/src/state/storage_runtime.rs), [postgres_runtime.rs](../../../edgequake/crates/edgequake-api/src/state/postgres_runtime.rs), [config.rs](../../../edgequake/crates/edgequake-api/src/state/config.rs), and [CLI bootstrap](../../../edgequake/src/main.rs).

Build three independently selected provider runtimes and a profile validator. Map absent new config to current P0. Keep provider role budgets, readiness probes and migration ownership. Add provider/version/binding health fields compatibly. Reject unknown providers, disabled features and unsupported combinations at startup.

**Exit:** T09/T19; production services receive ports; existing P0 config boots unchanged. No alternate-provider availability claimed yet. **Rollback:** restore the previous composition while schema and contracts remain compatible.

## PROVIDER-ACCESS-W3 — Complete relational ports and real commit boundaries

**Owner:** relational adapter/domain services. **Dependency:** W2. **Findings:** F02/F03/F10.

First complete J08: create the minimum receipt, canonical manifest, projection event and per-target delivery schema from [persistence recipes](persistence-and-provider-recipes.md#minimum-schema-before-any-new-writer). This schema precedes any new committer; W4 owns delivery/replay behavior rather than postponing schema required by W3. Review authority/transaction decisions at G2.

Replace the [DocumentRepository stub](../../../edgequake/crates/edgequake-storage/src/traits/domain/document_repository.rs) and label-only [UnitOfWork](../../../edgequake/crates/edgequake-storage/src/traits/domain/types.rs). Refactor [PG chunk repository](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/chunk_repository.rs) so document reservation, chunks, pending visibility and required durable intents use one private transaction executor. Return canonical DB IDs, including conflict-reused rows; never retain fabricated chunk IDs when the DB generated another UUID.

Move SQL from [relational_sidecar_store.rs](../../../edgequake/crates/edgequake-api/src/services/relational_sidecar_store.rs), [document_stage_mirror.rs](../../../edgequake/crates/edgequake-api/src/services/document_stage_mirror.rs), [workspace service](../../../edgequake/crates/edgequake-core/src/workspace_service_impl/mod.rs), and the PG entity sink behind injected domain ports. Remove process-global pool registration and `Box::leak`. Separate absent rows from provider errors. Preserve pure status/authorization policy in services.

Implement idempotent bounded commits and a publication manifest, plus expected-revision conflict handling. Keep multi-step transactions internal to the provider. Add a fault after parent insertion and before event append; assert no partial commit.

**Exit:** T03/T08/T11/T16; two runtimes with different stores remain independent; SQL dependency allowlist shrinks. Identity/conversation/queue persistence remaining for full relational substitution is explicitly tracked to W9. **Rollback:** additive schema remains; switch back only before new event authority or with proven compatible receipt handling.

## PROVIDER-ACCESS-W4 — Make graph/vector delivery recoverable

**Owner:** lifecycle/delivery. **Dependency:** W3. **Findings:** F04 plus authority-policy change D4.

Extend [outbox.rs](../../../edgequake/crates/edgequake-storage/src/outbox.rs), [outbox_drain.rs](../../../edgequake/crates/edgequake-storage/src/outbox_drain.rs), [outbox applier](../../../edgequake/crates/edgequake-api/src/services/outbox_drain_applier.rs), [ingestion persister](../../../edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs), and migration descriptors. Use the W3 schema; preserve telemetry events and implement isolated schema-versioned projection delivery, per-binding acknowledgment, immutable revision IDs, leases/fencing, quarantines and replay payload retention.

Complete replay/visibility over canonical graph contribution/revision storage and embedding manifests introduced in W3. Audit and reconcile existing AGE/spine divergence before enabling D4. Reuse compensation/quarantine for cleanup errors, but use tombstones/versioned intent as authoritative recovery. Implement current-state visibility batch reads and deletion binding persistence, including shared-fact suppression and late-write cleanup rules in [persistence recipes](persistence-and-provider-recipes.md#publication-shared-facts-and-stale-cleanup).

**Exit:** T03–T06/T10/T14/T17/T18; crash/replay from durable state restores graph/vector projections without LLM calls; poison events never disappear as success; no non-current content is served. **Rollback:** previous binding only after catch-up; retain canonical journal and tombstones.

## PROVIDER-ACCESS-W5 — Converge vector routing and model identity

**Owner:** vector/query. **Dependency:** W4. **Findings:** F06/F09/F11/F12.

Refactor [EmbeddingIndex](../../../edgequake/crates/edgequake-storage/src/traits/domain/embedding_index.rs), [FleetEmbeddingIndex](../../../edgequake/crates/edgequake-storage/src/traits/domain/fleet_embedding_index.rs), [workspace registry](../../../edgequake/crates/edgequake-storage/src/traits/workspace_vector.rs), PG typed adapters and [document vector resolver](../../../edgequake/crates/edgequake-api/src/services/document_vector_storage.rs). Add complete typed requests and model descriptors; treat model IDs as authoritative. Route all four families through the same scoped search/mutation semantics, preserving specialized storage layouts.

Move name/legacy-key hydration into one compatibility adapter, outside vendor-specific search semantics. Keep current typed pgvector layout for co-located P0; add a migration-owned standalone embedding projection layout with scoped canonical IDs, family, model/content revision and indexed filter payload for external relational authorities. Its schema has no FK to a remote authority, and visibility/hydration use injected relational ports. Validate layout compatibility at composition and test SQLite + Neo4j + standalone pgvector in W9. Deprecate table-name/drop methods in favor of index-binding lifecycle. Extract sparse text search capability. Replace default-storage deletion fallback with pending targeted cleanup. Remove old facade call sites after coverage proves equivalent behavior.

**Exit:** T01/T02/T07/T10/T13/T15; no filter/model loss, no default deletion routing, no dropped legacy-store recreation. **Rollback:** binding-based with retained current data, never an unconditional legacy flag flip.

## PROVIDER-ACCESS-W6 — Require bounded graph operations

**Owner:** graph/query. **Dependency:** W5. **Findings:** F07/F08.

Retain [graph ISP traits](../../../edgequake/crates/edgequake-storage/src/traits/graph_isp.rs); complete scoped typed keys and bounded adjacency/scan ports. Adapt [PG graph implementation](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs) and [memory graph](../../../edgequake/crates/edgequake-storage/src/adapters/memory/graph.rs). Replace production inherited full scans/per-node RPC defaults. Reuse [dedupe helpers](../../../edgequake/crates/edgequake-storage/src/graph_batch_dedupe.rs); enforce MergeSources versus Replace and complete contribution lineage. Optimize memory vector top-k without changing score semantics.

**Exit:** T02/T05/T12/T15; dense/cyclic multigraph traversal terminates with truthful truncation; operation-count gates hold. **Rollback:** retain existing native indexed implementation where equivalent; no full-graph fallback on a hot path.

## PROVIDER-ACCESS-W7 — Prove vector substitution with Qdrant

**Owner:** Qdrant adapter. **Dependency:** W6. **Findings addressed by proof:** F01/F05/F06/F09/F11/F14.

Add proposed Qdrant adapter module/crate, feature, factory registration and explicit collection/index provisioning. Use UUID point IDs for full model/content revisions, mandatory scope/family/document payload fields and their indexes. Persist receipts and implement configured wait/consistency/ordering policies; do not infer completion from HTTP acceptance. Parameterize all four vector families. Read current official docs and pin a server image plus compatible client at implementation time.

**Exit:** P1 passes T01/T04/T06/T07/T10/T13–T15/T17/T20 and end-to-end upload/query/delete/restart; chaos proves stale delivery cannot overwrite current content. **Rollback:** P0 vector binding only after catch-up. No production provider switch solely because unit tests pass.

## PROVIDER-ACCESS-W8 — Prove graph substitution with Neo4j

**Owner:** Neo4j adapter. **Dependency:** W7. **Findings addressed by proof:** F01/F07/F08/F12/F14.

Add proposed Neo4j adapter, feature, registration and migrations. Use application UUID properties and composite scope/revision lookup indexes, parameterized batch UNWIND, explicit edge-type/direction identity, bounded adjacency and revision-aware contribution updates. Avoid treating AGE-specific agtype SQL as portable Cypher. The initial implementation uses single-node Query API over the existing HTTP client and the revision-node mapping in [persistence recipes](persistence-and-provider-recipes.md#provider-recipes); qualify server/version/license support. Validate response-body errors; cluster affinity/bookmark certification is required before any future cluster support ([S12](../references/official-sources.md#s12-neo4j-query-api)).

**Exit:** P2 passes graph/visibility tests T01/T02/T04–T06/T12/T14/T17/T20, including Unicode keys, loops, sibling relationships and cancellation. Complete graph replay from canonical facts is mandatory. **Rollback:** retain/catch up AGE binding before switching back.

## PROVIDER-ACCESS-W9 — Prove relational substitution and retire compatibility debt

**Owner:** relational/API/tasks. **Dependency:** W8. **Findings:** F02/F03/F10/F12/F14.

Implement SQLite provider in the proposed adapter module/crate with explicit migrations and bounded single-writer transactions. Port remaining SQL from [identity storage](../../../edgequake/crates/edgequake-api/src/services/identity_storage.rs), [session storage](../../../edgequake/crates/edgequake-api/src/services/session_storage.rs), conversations, membership/quota services and [task persistence](../../../edgequake/crates/edgequake-tasks/src/postgres.rs) into adapters or existing port implementations. Cover provider-budget persistence, checkpoints, artifacts, binary originals/PDFs, migration jobs, auth revocation and audit data in the profile manifest; a missing required port rejects boot.

Reconcile PG-specific JSON, timestamp, boolean, uniqueness/NULL, pagination and lease behavior with contract tests. Preserve PG RLS defense in depth; SQLite uses mandatory scoped predicates and file access controls rather than pretending to provide RLS. Make `DATABASE_URL` mandatory only for provider configurations requiring PostgreSQL. Reject P3 multi-replica deployment until separately certified.

Remove old `UnitOfWork`, global sidecar pool, duplicated vector routing and application-layer PG dependencies after callers move. Complete binding backfill/shadow/soak/rollback rehearsal from [consistency and migration](../design/consistency-and-migration.md). Adapter-specific diagnostics remain optional and privileged.

**Exit:** P3 boots without a PG relational service and passes T01–T20 as applicable plus auth/upload/query/delete/restart/recovery; P0 remains green. Dependency audit has zero unallowlisted driver/SQL references outside adapters/composition/migration tooling. **Rollback:** restore retained PG canonical authority only through verified replication/export reconciliation; no claim of automatic relational failover.

## Final implementation definition of done

All invariants I1–I8 have runtime tests; all findings have evidence of closure; all six compositions P0/P1/P2a/P2b/P3/P4 are certified as defined in [feasibility assessment](feasibility-and-releases.md). Interim releases may leave later profiles unavailable; full PROVIDER-ACCESS completion may not. Provider data cannot silently cross scope; physical cleanup is recoverable; benchmarks disclose tradeoffs; rollback/restore is rehearsed. [definition of done](../validation/definition-of-done.md) is the complete acceptance checklist. Proposed commands/tests become required when added by W0; they are not available merely because this plan names them.

Documentation completion in this task is distinct from implementation completion. [contract validation](../validation/edge-cases-and-contract-tests.md) records this audit's actual validation status.
