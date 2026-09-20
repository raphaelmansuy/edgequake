# WHY the current abstractions do not yet guarantee substitution

Audit baseline and scope are in the [entry point](../README.md). Code was traced from runtime composition through storage traits, PostgreSQL/memory adapters, domain repositories, ingestion persistence, query bridges, and operational services. No running-database security exploit, latency regression, or full-repository correctness claim is implied.

**Severity convention:** P1 = correctness/isolation/recovery contract requiring resolution before another provider is enabled; P2 = structural/performance/verification limitation. “Observed” means visible in source. “Extension risk” means an unsafe default exists; it does not mean every shipped adapter inherits it.

## What already works and must survive

- Graph traits are already split into read, mutation, scan, and analytics capabilities (E27). Do not propose interface segregation as if absent.
- Typed `ChunkRepository`, `EmbeddingIndex`, `FleetEmbeddingIndex`, workspace registries, and conversation/blob ports already exist. Reuse and complete them.
- Native PostgreSQL operations use bulk arrays/UNNEST, scoped indexes, role-specific pools (E33), and typed vector tables. Keep those optimizations behind adapters.
- Graph deduplication already uses hash maps with stable first-seen key order and last payload wins (E22). Production PostgreSQL and memory adapters override type-specific batch edge deletion (E25–E26).
- Serving fences, compensation/quarantine, migration ledgers, DATA-* tracing, and backend contracts exist. Their presence does not establish a durable cross-provider transaction or complete conformance.

## Findings

| ID | Priority / classification | Evidence and failure mechanism | Required change |
|---|---|---|---|
| PROVIDER-ACCESS-F01 | P1, observed coupling | E01–E03: one Memory/PostgreSQL mode; bootstrap directly constructs PG KV/vector/AGE; `PostgresRuntime` exports a pool | Independent validated provider composition; W2 |
| PROVIDER-ACCESS-F02 | P1, observed incomplete relational boundary | E04/E28/E29: document port defaults to no-op; workspace/identity code uses PG pools/SQL. Text scan finds 89 API and 8 core source files referencing SQLx/PgPool | Complete domain repositories and operational ports; W3/W9 |
| PROVIDER-ACCESS-F03 | P1, observed atomicity gap | E05–E06: `UnitOfWork` is only `label`; PG chunk insert/delete ignore `_tx`; parent ensure and chunk insert use pool statements | Adapter-private real transaction around bounded domain commits; W3 |
| PROVIDER-ACCESS-F04 | P1, observed delivery limitation | E07–E08: outbox enqueue uses independent pool INSERT; best-effort helper swallows failure; milestone events are acked, unknown kinds also acked | Separate durable projection events, atomic append, versioned dispatch, quarantine unknown schemas; W4 |
| PROVIDER-ACCESS-F05 | P1, extension risk + observed permissive helper | E09–E10: default filtered search ignores filter; missing tenant/workspace metadata passes `matches_fields`; helper tests explicitly accept `{}` | Mandatory scope and required filter semantics, legacy rows quarantined/reconciled; W1 |
| PROVIDER-ACCESS-F06 | P1, observed typed-route contract gap | E13–E15/E24/E30: typed branch passes only embedding/k/workspace to a query without document, modality, tenant or ID fields; returns before later fence; errors become empty success | Preserve all predicates and visibility in typed route; errors remain errors; W1/W5 |
| PROVIDER-ACCESS-F07 | P1, extension risk | E11/E32: default triple-edge delete collapses to pairs; graph write-mode defaults ignore mode; vector clear/document-delete return zero | Required exact mutation semantics; certify explicit overrides; W1 |
| PROVIDER-ACCESS-F08 | P2, extension/performance risk | E12 and read-trait defaults: induced-edge query loads all edges; degree/incident fallbacks loop RPCs; `ping` defaults to count. E23: memory vector search sorts every candidate | Required bounded production methods; top-k selection; independent operation budgets; W1/W6 |
| PROVIDER-ACCESS-F09 | P1, observed model-contract mismatch | E15/E24/E31: `ModelId` is accepted but PG adapter resolves its own configured name+dimension; typed upsert is `DO NOTHING` on conflict | Model revision is authoritative; distinguish immutable insert/idempotent replay from replacement; W5 |
| PROVIDER-ACCESS-F10 | P2, observed dependency/lifecycle coupling | E16: process-global sidecar pool is replaced on registration and leaked for `'static`; getters merge errors and absence | Inject owned repositories per runtime; `Result<Option<T>>`; W3 |
| PROVIDER-ACCESS-F11 | P1, observed destructive-path fallback | E17: any strict workspace resolution error during deletion routes to default vector storage | Persist provider binding for deletion; tombstone and retry the correct provider; W5 |
| PROVIDER-ACCESS-F12 | P2, observed vendor details in public contracts | E18–E19/E34: typed vectors force PG relational sink; workspace registry exposes table naming/drop; `VectorBackend` selects schema generation, not vendor | Shared typed IDs and index bindings; keep migration compatibility adapter-local; W2/W5 |
| PROVIDER-ACCESS-F13 | P2, observed error classification weakness | E20: SQLx mapping infers uniqueness from constraint-name substrings and collapses other errors to strings | Classify SQLSTATE/status, retriable versus unknown outcome; W1 |
| PROVIDER-ACCESS-F14 | P1, verification gap | E21: PostgreSQL contract tests can return success after skipping absent configuration; current suites cover portions of semantics | Required service-backed certification jobs that fail for absent fixtures; W0/W7–W9 |

F06 proves that the storage boundary does not enforce its accepted predicates. It does **not** prove every API response leaks data: callers may add restrictions. A caller-side filter also cannot recover relevant matches excluded by an earlier LIMIT, and cannot repair the abstraction's safety contract. The immediate regression must exercise the typed storage branch directly and then the user-facing route.

F09 is a mismatch between the port's apparent model identity and adapter behavior, not a claim that deployed callers currently pass distinct model IDs. F07's dangerous edge fallback is **overridden by both current adapters**; preserve those fixes when adding providers.

## Current dependency and write shape

```ascii
API / services ----> PostgresRuntime / sidecar global ----> SQLx pool
      |
      +----> StorageRuntime: KV + GraphStorage + VectorStorage
      |                           |                  |
      |                          AGE           PG typed/legacy bridge
      |                                              |
      +----> PG entity sink <-------------------------+
      |
Pipeline --> ChunkRepository(_tx label) --> pool statements
      |
      +----> graph/vector writes --> best-effort milestone outbox
```

A trait object at the outer edge does not remove the provider assumptions behind it. The data path also carries two vector APIs and legacy-string conversions. [architecture](architecture.md) defines the eventual single ownership; [work packages](../implementation/work-packages.md) avoids a big-bang rewrite.

## Reproducible footprint, not a count of defects

At the pinned commit, a literal scan of `edgequake/crates/<crate>/src/**/*.rs` for `sqlx::|\bPgPool\b` yields:

| Crate | Rust files | Matching files |
|---|---:|---:|
| edgequake-api | 477 | 89 |
| edgequake-core | 66 | 8 |
| edgequake-pipeline | 93 | 1 |
| edgequake-query | 82 | 0 |
| edgequake-tasks | 43 | 4 |
| edgequake-storage | 167 | 69 |

This includes comments/tests/legitimate adapters; the pipeline hit is a comment saying it must not depend on `PgPool`. Use the inventory to assign moves, not as an architectural failure threshold. W0 creates a reviewed import/SQL allowlist; services then shrink that allowlist to zero while composition and adapters retain legitimate vendor dependencies.

## Evidence register

The following anchors are generated from the inspected checkout. Hashes and exact needle strings live in [code-evidence.json](../evidence/code-evidence.json). Lines are evidence at the baseline, not a promise about future HEAD.

| Anchor | Source | Line / symbol |
|---|---|---|
| E01 | [state/config.rs](../../../edgequake/crates/edgequake-api/src/state/config.rs) | 23: `pub enum StorageMode` |
| E02 | [state/postgres.rs](../../../edgequake/crates/edgequake-api/src/state/postgres.rs) | 365: `let graph_storage = Arc::new` |
| E03 | [state/postgres_runtime.rs](../../../edgequake/crates/edgequake-api/src/state/postgres_runtime.rs) | 12: `pub pool: Option<PgPool>` |
| E04 | [traits/domain/document_repository.rs](../../../edgequake/crates/edgequake-storage/src/traits/domain/document_repository.rs) | 12: `async fn touch_indexed` |
| E05 | [traits/domain/types.rs](../../../edgequake/crates/edgequake-storage/src/traits/domain/types.rs) | 92: `pub struct UnitOfWork` |
| E06 | [adapters/postgres/chunk_repository.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/chunk_repository.rs) | 267: `_tx: &mut UnitOfWork` |
| E07 | [outbox.rs](../../../edgequake/crates/edgequake-storage/src/outbox.rs) | 96: `pub async fn enqueue_outbox_best_effort` |
| E08 | [services/outbox_drain_applier.rs](../../../edgequake/crates/edgequake-api/src/services/outbox_drain_applier.rs) | 43: `async fn apply_outbox_event` |
| E09 | [traits/vector.rs](../../../edgequake/crates/edgequake-storage/src/traits/vector.rs) | 404: `async fn query_filtered` |
| E10 | [traits/vector.rs](../../../edgequake/crates/edgequake-storage/src/traits/vector.rs) | 151: `fn matches_fields` |
| E11 | [traits/graph_mutate_ops.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph_mutate_ops.rs) | 119: `async fn delete_edges_batch` |
| E12 | [traits/graph_read_ops.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph_read_ops.rs) | 209: `async fn get_edges_for_node_set` |
| E13 | [adapters/postgres/vector/storage_impl.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs) | 714: `async fn query_filtered` |
| E14 | [adapters/postgres/vector/typed_read.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector/typed_read.rs) | 117: `pub async fn try_typed_chunk_query` |
| E15 | [adapters/postgres/chunk_embedding_index.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/chunk_embedding_index.rs) | 150: `async fn search` |
| E16 | [services/relational_sidecar_store.rs](../../../edgequake/crates/edgequake-api/src/services/relational_sidecar_store.rs) | 32: `static SIDECAR_POOL` |
| E17 | [services/document_vector_storage.rs](../../../edgequake/crates/edgequake-api/src/services/document_vector_storage.rs) | 69: `pub async fn get_workspace_vector_storage_for_delete` |
| E18 | [postgres_entity_sink.rs](../../../edgequake/crates/edgequake-api/src/postgres_entity_sink.rs) | 49: `pub async fn create_for_runtime` |
| E19 | [traits/workspace_vector.rs](../../../edgequake/crates/edgequake-storage/src/traits/workspace_vector.rs) | 85: `pub fn table_name` |
| E20 | [error.rs](../../../edgequake/crates/edgequake-storage/src/error.rs) | 68: `impl From<sqlx::Error>` |
| E21 | [edgequake/crates/edgequake-storage/tests/storage_backend_contract.rs](../../../edgequake/crates/edgequake-storage/tests/storage_backend_contract.rs) | 50: `Skipping postgres contract` |
| E22 | [graph_batch_dedupe.rs](../../../edgequake/crates/edgequake-storage/src/graph_batch_dedupe.rs) | 26: `pub fn dedupe_nodes_by_id` |
| E23 | [adapters/memory/vector.rs](../../../edgequake/crates/edgequake-storage/src/adapters/memory/vector.rs) | 126: `scores.sort_by` |
| E24 | [traits/domain/types.rs](../../../edgequake/crates/edgequake-storage/src/traits/domain/types.rs) | 98: `pub struct VectorQuery` |
| E25 | [adapters/postgres/graph/graph_storage_impl.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/graph_storage_impl.rs) | 253: `async fn delete_edges_batch` |
| E26 | [adapters/memory/graph.rs](../../../edgequake/crates/edgequake-storage/src/adapters/memory/graph.rs) | 922: `async fn delete_edges_batch` |
| E27 | [traits/graph.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph.rs) | 180: `pub trait GraphStorage:` |
| E28 | [workspace_service_impl/mod.rs](../../../edgequake/crates/edgequake-core/src/workspace_service_impl/mod.rs) | 44: `pool: PgPool` |
| E29 | [services/identity_storage.rs](../../../edgequake/crates/edgequake-api/src/services/identity_storage.rs) | 119: `pool: &sqlx::PgPool` |
| E30 | [adapters/postgres/serving_fence_query.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/serving_fence_query.rs) | 29: `pub async fn apply_serving_fence` |
| E31 | [adapters/postgres/chunk_embedding_index.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/chunk_embedding_index.rs) | 94: `_model: ModelId` |
| E32 | [traits/graph_mutate_ops.rs](../../../edgequake/crates/edgequake-storage/src/traits/graph_mutate_ops.rs) | 54: `async fn upsert_nodes_batch_with_mode` |
| E33 | [adapters/postgres/pool_bundle.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs) | 73: `pub struct PgPoolBundle` |
| E34 | [vector_backend.rs](../../../edgequake/crates/edgequake-storage/src/vector_backend.rs) | 5: `pub enum VectorBackend` |


## Relationship to earlier work

[021](../../021-storage-study/README.md) provides historical architecture context; [088](../../088-data-layer/README.md) owns operation inventory and measured PG optimizations; [091](../../091-simplify-data-layer/README.md) introduced typed ports and cutover policy; [098](../../098-data-access-hardening/README.md) protects lineage, typed fleet FKs, and multigraph deletion. PROVIDER-ACCESS extends these contracts across providers. Historical “done” statements are not substituted for the current source inspection.
