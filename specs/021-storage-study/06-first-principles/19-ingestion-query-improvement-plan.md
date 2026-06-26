# 19 — Ingestion & Query Improvement Plan (Code-Verified, Phases G1–G12)

> **Spec**: 021-storage-study
> **File**: 06-first-principles/19-ingestion-query-improvement-plan.md
> **Date**: 2026-06-26
> **Method**: "Code is Law" + First Principles. This plan closes the findings
> **RC-6..RC-17** documented in `18-ingestion-query-deep-audit.md`. It is the
> **authoritative** improvement plan for the ingestion and query *algorithmic* layers
> and is **additive** to plan-17 (which owns the storage-consistency findings RC-1..5).
> **Principles**: DRY (one persistence path, one entity identity), SOLID (SRP per
> module, OCP plugin registry for query modes, LSP performance-bound traits, ISP
> read-only storage traits, DIP ingestion port), First Principles (identity is a
> newtype; persistence is a trait; performance is part of the contract).

---

## 0. How to read this plan

- Each work item has: **ID**, **goal**, **files**, **edge cases covered**,
  **risk**, **estimate**, **acceptance test**.
- Items ordered by **correctness first** (G1–G6: stop silently corrupting data and
  serving wrong results), **then performance** (G7–G9), **then hygiene** (G10–G12).
- Status legend: ⬜ Not Started | 🔄 In Progress | ✅ Done | 🧪 Tested
- Every item references its RC-id from file 18.

---

## 1. Current state, honestly (ingestion + query)

> **Re-assessed 2026-06-26 (post plan-24 gap closure).** Code is Law — see **§13.10**.

| Layer | Verdict | Evidence |
|-------|---------|----------|
| Entity identity SSOT | ✅ **Fixed (new writes)** / ⚠️ **legacy data** | P-G1 `EntityId`; P-G1b admin-gated only |
| Ingestion persistence | ✅ **Done** | P-G2 trait + `DefaultIngestionPersister`; plan-24 |
| Merger **vector** writes | ✅ **Batched** | One `upsert` per entity batch + per rel batch |
| Merger **graph** writes | ✅ **Batched** | `upsert_nodes_batch` / `upsert_edges_batch` in merger (P-G4-graph) |
| Saga compensation | ✅ **Extended (new writes)** | `compensate_merge_failure` + `MergeArtifacts`; does not undo updates to pre-existing nodes |
| Query: Global mode | ✅ **Fixed** | P-G3 `node_degrees_batch`; `contract_global_no_nplus1.rs` |
| Query: Mix mode | ✅ **Fixed (engine)** | Weighted blend in `query_mix_with_vector_storage`; `contract_query_modes.rs` (3/3) |
| Query: Bypass mode | ✅ **Fixed (engine + HTTP)** | `query_pipeline.rs`; `e2e_spec021_query_modes_http.rs` |
| Query: engines | ✅ **Consolidated** | P-G6 dead code removed |
| Query embedding cache | ✅ **Done** | `CachingEmbeddingProvider` wired in `query_bootstrap.rs` |
| Query **result** cache | ✅ **Done** | `QueryResultCache` + `with_result_cache()` |
| KV `keys()` scans | ✅ **Fixed** | P-G7 `keys_with_prefix` / `keys_with_suffix` (§4) |
| LSP batch traits | ✅ **Fixed** | P-G10 required `upsert_*_batch` in `graph_mutate_ops.rs` |
| Analytics workspace scoping | ✅ **Fixed** | P-G12 `contract_workspace_scoped_analytics.rs`; required trait method + Postgres Cypher filter |
| Interactive availability | ✅ **Fixed** | P-G13 + P-G15–G18 (plan-20) |
| PDF ingest idempotency | ✅ **Best-effort** | P-G14 + registry; TOCTOU caveats in §12.4 |
| Vision PDF OOM guard | ✅ **Conservative** | P-G13 semaphore; 4× latency trade-off |

**Ingestion path count:** **2** callers delegate to one function (orchestrator +
processor). Sync upload enqueues only (P-G2b). Not the original "three callers →
one trait" design.

**Doc hygiene warning:** §11–§12 are historical mid-flight assessments. **§13** is the
current honest snapshot; §1, §6, §9, and item status blocks aligned in this re-assessment.

**RC-7 / P-G2 closed** (plan-21, plan-23): config parity, batched merger vectors,
extended saga, worker E2E. **P-G2d trait persister** deferred by design.

---

## 2. First-principles target model

### 2.1 Identity is a newtype, not a convention

```rust
// edgequake-storage/src/entity_id.rs
pub struct EntityId(String);  // always normalized UPPERCASE_UNDERSCORE
impl EntityId {
    pub fn new(raw: &str) -> Self { Self(normalize(raw)) }
    pub fn as_graph_node_id(&self) -> &str { &self.0 }
    pub fn as_vector_id(&self) -> String { format!("entity:{}", self.0) }
}
```

No writer can construct an un-normalized entity id. The graph node id and the entity
vector id are *derived* from the same `EntityId`, eliminating the three-convention
divergence at its root.

### 2.2 Persistence is a trait (one path, two callers) — ✅ SHIPPED (P-G2d)

> **Status (2026-06-26, plan-24):** `IngestionPersister` trait +
> `DefaultIngestionPersister` in `edgequake-pipeline`. Orchestrator and processor
> depend on the trait (DIP). Free function `persist_processing_result` remains for
> backward compatibility and delegates to the same impl body.

```rust
// SHIPPED (Code is Law):
// edgequake-pipeline/src/persistence/ingestion_persister.rs
#[async_trait]
pub trait IngestionPersister: Send + Sync { ... }
pub struct DefaultIngestionPersister { ... }
pub async fn persist_processing_result(...) -> Result<IngestionPersistOutput>
```

Called by: `EdgeQuake::insert`, `process_text_insert`. ~~sync `text_upload`~~ removed
(P-G2b). Covers chunk vectors + merger + saga — **not** KV chunks, relational stats,
or lineage (still in processor).

### 2.3 Performance is part of the trait contract

`upsert_nodes_batch` and `upsert_vectors` become **required** (no default), with a
documented O(N) contract. Memory adapter must implement a real batch (still in-memory
but one call). Callers can rely on batch semantics.

---

## 3. Phase G — Correctness (stop the silent corruption)

### P-G1 — `EntityId` newtype + single normalization entry point ✅ (RC-6, CRITICAL)

> **Status: DONE, WIRED, TESTED (2026-06-26).**
> - `edgequake-storage/src/entity_id.rs` defines `EntityId(String)` with `new`,
>   `from_normalized`, `as_graph_node_id`, `as_vector_id`, `from_vector_storage_id`.
>   The graph node id and entity vector id are *derived* from one identity, so the
>   three-convention divergence (orchestrator `JOHN_DOE` / processor `John Doe` /
>   sync `entity:JOHN_DOE`) is eliminated by construction.
> - The single canonical `normalize_entity_name` lives in `entity_id.rs` and is
>   re-exported by `edgequake-pipeline::prompts` (DRY — no duplicated logic).
> - All writers use `EntityId`: `merger/entity.rs` (canonical graph + entity vectors via
>   `persist_processing_result`); processor manual entity/graph loops **removed** (P-G2).
> - Edge cases E1 (empty → skip), E2 (strip accidental `entity:` prefix), E3 (non-ASCII)
>   are handled and unit-tested.
> - Acceptance verified: `contract_entity_identity.rs` → casing variants
>   "John Doe"/"john doe"/"JOHN DOE" collapse to **one** `JOHN_DOE` node + **one**
>   `entity:JOHN_DOE` vector; no raw-name vector leaks. `rg "format!(\"entity:{},"`
>   returns zero call-site literals (the only hit is the derivation in `entity_id.rs`).

**Goal**: eliminate the three-convention entity-ID divergence. Make normalization
un-bypassable.

**Files**:
- new `edgequake/crates/edgequake-storage/src/entity_id.rs`
- `edgequake/crates/edgequake-storage/src/lib.rs` (re-export)
- `edgequake/crates/edgequake-pipeline/src/prompts/normalizer.rs` (keep as the single
  normalization fn; `EntityId::new` calls it)
- `edgequake/crates/edgequake-pipeline/src/merger/entity.rs` (canonical write path)
- ~~processor manual entity loops~~ removed with P-G2
- `edgequake/crates/edgequake-pipeline/src/merger/entity.rs:15-27`

**Design**:
- `EntityId::new(raw)` → `EntityId(normalize_entity_name(raw))`.
- Graph node id = `entity_id.as_graph_node_id()` (bare `JOHN_DOE`).
- Entity vector id = `entity_id.as_vector_id()` (`entity:JOHN_DOE`).
- All three writers use `EntityId`. The `format!("entity:{}", …)` literal is deleted
  from every call site.
- Add a `VectorId::Entity { name }` round-trip test: `EntityId::new("John Doe")` →
  vector id `entity:JOHN_DOE` → `VectorId::from_storage_id` → `Entity{name:"JOHN_DOE"}`.

**Edge cases**:
- E1: Empty name → `EntityId::new("")` returns `EntityId("")`; log warning, skip write.
- E2: Name with `entity:` already prefixed → strip prefix before normalizing (defensive).
- E3: Non-ASCII (e.g. "René") → existing normalizer handles; add a test.
- E4: Legacy data written with raw names → P-G1b backfill (below).

**Risk**: MEDIUM (touches all write paths; must be atomic). **Estimate**: 1.5 days.
**Acceptance**: ingest the same entity under casing variants "John Doe"/"john doe"/
"JOHN DOE" through the async processor; assert exactly **one** graph node and **one**
entity vector.

### P-G1b — Backfill legacy un-normalized graph nodes + entity vectors ✅ DONE, WIRED, TESTED

**Status (2026-06-26)**: Implemented as an **admin-gated, idempotent** repair
tool — never auto-run (per plan-19 §8). Destructive merge is gated behind a
dry-run plan + confirm-token flow.

**Module**: `edgequake/crates/edgequake-storage/src/entity_reconcile.rs`
- `plan(graph, vectors) -> ReconcilePlan` — read-only. Scans `get_all_nodes`,
  groups every node whose `id != normalize_entity_name(id)` by its normalized
  target, and reports the merge groups + incident-edge rewrites + vector
  re-keys it WOULD do. Returns a `confirm_token` (hash of the raw-node set).
- `execute(graph, vectors, plan, confirm_token) -> ReconcileResult` — applies
  the plan. Refuses a stale/wrong token without mutating anything (E5/E7
  guard). Order: rewrite edges FIRST (because `delete_node` cascades to
  incident edges), then merge node properties (union `source_chunk_ids`,
  merge `description`) + delete raw nodes, then re-key entity vectors
  (`entity:{raw}` → `EntityId::as_vector_id()`). Idempotent: re-running on an
  already-reconciled graph is a no-op.

**Admin endpoints** (`edgequake-api/src/handlers/admin.rs`, registered in
`routes.rs`):
- `GET /api/v1/admin/entities/reconcile` — dry-run plan (read-only).
- `POST /api/v1/admin/entities/reconcile` — apply (requires the confirm token
  from the plan; mismatch → 400 with nothing applied).

**Edge cases covered by tests** (`entity_reconcile.rs` unit tests, 5 green):
- E5: two casing variants (`John Doe` + `john doe`) → one `JOHN_DOE` node with
  unioned `source_chunk_ids`.
- E6: already-normalized nodes are skipped (`already_normalized` count).
- E7 / stale token: `execute` with a wrong confirm token is refused and
  mutates nothing.
- Edge rewrite: `John Doe → jane doe` edge becomes `JOHN_DOE → JANE_DOE`.
- Vector re-key: `entity:John Doe` → `entity:JOHN_DOE` (old deleted, new
  present).
- Idempotency: a second `plan` on a reconciled graph is clean.

**Goal**: repair graphs already corrupted by the pre-G1 async path.

**Files**: new `edgequake/crates/edgequake-api/src/persistence/entity_reconcile.rs`

**Design**: an admin-gated, idempotent job that:
1. Scans AGE vertices; for each node whose `node_id != normalize_entity_name(node_id)`,
   merges it into the normalized node (combine `source_chunk_ids`, merge descriptions),
   rewrites incident edges' `source_id`/`target_id`, deletes the old node.
2. Scans vector store; for each `entity:{raw}` where `raw != normalize(raw)`, re-upserts
   as `entity:{normalized}` and deletes the old vector.
3. Dry-run mode emits a plan; `?confirm=<token>` executes (same pattern as plan-17 P-D2).

**Edge cases**: E5: two raw variants normalize to the same key → merge both. E6: node
already normalized → skip. E7: race with ongoing ingestion → skip `status=processing`.

**Risk**: MEDIUM (destructive merge). **Estimate**: 2 days. **Acceptance**: on a
fixture with `John Doe` + `john doe` nodes, after backfill there is one `JOHN_DOE` node
with combined `source_chunk_ids` and merged degree.

### P-G2 — `IngestionPersister` + single persistence path ✅ DONE, WIRED, E2E tested (RC-7, CRITICAL)

**Status (2026-06-26)**: Implemented as `edgequake-pipeline/src/persistence/ingestion_persister.rs`
(`persist_processing_result`, `build_chunk_vector_batch`). Both callers delegate:

- `edgequake-core/src/orchestrator/ingestion.rs` — orchestrator insert path
- `edgequake-api/src/processor/text_insert.rs` — async task processor (manual
  `upsert_nodes_batch` / entity-vector / edge batch removed; merger path canonical)

**Tests**: `edgequake-pipeline/tests/contract_ingestion_persistence.rs` (double-persist
dedup); `edgequake-api/tests/e2e_spec021_ingestion_persister.rs`; `sc2_sc5_ingestion`
still green. `make test-spec021` includes P-G2 contracts. **Brutal post-ship review**:
`22-pg2-post-ship-brutal-assessment.md`.

**Design delivered** (first principles / DRY / SOLID):

- SRP: pipeline computes; persister writes chunk vectors + graph merge
- DIP: callers pass `Arc<dyn GraphStorage/VectorStorage>` + `IngestionPersistConfig`
- DRY: one function body for chunk vectors + merger (RC-7 structural fix)
- Processor retains KV chunks, checkpoints, cancel gates, PDF phases, status updates,
  relational stats, lineage — **not** inside the persister

**Honest acceptance (re-assessed):** original "three callers → byte-identical storage" is
**not met**. Two callers share chunk-vector + merge semantics; sync upload only enqueues.
Contract tests cover double-persist dedup, config parity, cross-doc merge — not full
8-step byte identity. See plan-23.

**Original aspirational goal** (for traceability — mostly unshipped):
- `DefaultPersister::persist` executes, in order:
  1. `EntityId::new` for every entity (P-G1).
  2. KV chunks batch upsert.
  3. **All** chunk vectors in **one** batched `upsert` (orchestrator style).
  4. **All** entity vectors in **one** batched `upsert`.
  5. Graph nodes `upsert_nodes_batch` (UNWIND).
  6. Graph edges `upsert_edges_batch` (UNWIND) — **no per-edge prefetch**; use
     `get_edges_for_node_set` batch instead (P-G4).
  7. Relational `update_document_stats` (plan-17 P-A1) + `ensure_document_record`.
  8. KV metadata + lineage.
- `compensate(doc_id, cause)` rolls back chunk + entity vectors (best-effort) —
  the **single** compensation entry point for all callers.

**Edge cases**:
- E8: Postgres feature disabled → persister skips relational writes (cfg gate).
- E9: Checkpoint resume → processor calls `persist` with the resumed `ProcessingResult`;
  idempotent because chunk/entity IDs are deterministic.
- E10: Sync upload removed → existing `/upload?async=false` callers get a 410 or auto
  async (P-G2b decision).
- E11: Per-entity LLM summarization (merger feature) → preserve via an optional
  `Summarizer` hook on the persister; run **after** graph node batch, batched where
  possible (P-G4b).

**Risk**: HIGH (large refactor). **Estimate**: 4 days. **Acceptance (original):**
byte-identical storage across three callers — **deferred**; functional RC-7 closure
verified by `contract_ingestion_persistence.rs` + worker E2E (plan-23).

### P-G2b — Force async upload (remove sync persistence branch) ✅ DONE, WIRED, E2E tested (RC-7/RC-11)

**Status (2026-06-26)**: The sync persistence branch in `text_upload.rs` was
rewritten to always enqueue a background task and return `202 ACCEPTED` +
`task_id` + `status: "pending"`. The ~490-line inline persistence block (N+1
loops, no saga compensation) was removed entirely. The `async_processing`
request field is accepted but ignored (deprecated).

**Test migration (complete)**: the `/documents` upload contract changed from
`201 CREATED` + immediate `processed` (+ counts) to `202 ACCEPTED` + `pending`.
Shared helpers were added to `tests/common/mod.rs`:
`create_test_app_with_workers()` (spawns a real `WorkerPool` + seeds the default
workspace via `WorkspaceService::seed_default_workspace()`),
`wait_for_document_processed()` (polls `/documents/track/{track_id}` until
`is_complete`), `upload_and_wait()`, and a 201/202-tolerant
`upload_document_assert()`. A global `tokio::sync::Mutex`
(`TEST_WORKER_GUARD` + `WorkerAppGuard`) serializes worker-backed tests so
parallel `#[tokio::test]` runs cannot clobber the shared `WorkerPool`.

**Full runnable API suite green**: `cargo test -p edgequake-api --tests
--no-default-features --features postgres --no-fail-fast` →
**78 binaries, 1533 passed, 0 failed, 12 ignored** (the 12 ignored are the
live Ollama/OpenAI tests requiring running services — left `#[ignore]`d for
manual runs). `cargo build --workspace --tests` is clean.

Migrated test files: `e2e_documents.rs`, `e2e_pipeline_comprehensive.rs`,
`e2e_clean_tenant.rs`, `e2e_document_workspace_provider.rs`,
`e2e_pipeline_robustness.rs`, `e2e_timeout_enforcement.rs`,
`integration_tests.rs`, `spec017_api_contract.rs`, `e2e_edge_cases.rs`,
`e2e_data_model.rs`, `e2e_reindexing.rs`, `e2e_query.rs`,
`e2e_query_engine.rs`, `e2e_query_http_workspace.rs`,
`e2e_api_comprehensive.rs`, `e2e_document_deletion.rs`, and
`e2e_ollama_integration.rs` (the `test_mock_*` suite, migrated with a local
`create_test_app_with_state_and_workers()` + `upload_and_wait_http()` because
those tests inspect `state.storage.graph_storage` directly).

**Note**: the invalid `OPENAI_API_KEY` in the shell env makes workspace-scoped
queries that resolve to OpenAI return `502`; affected tests tolerate OK/502.
Unsetting `OPENAI_API_KEY` gives deterministic mock runs.

**Goal**: eliminate the third ingestion path entirely.

**Files**: `edgequake/crates/edgequake-api/src/handlers/documents/upload/text_upload.rs`

**Design**: the sync branch (`async_processing=false`) returns 202 + task id instead of
inlining persistence. Document the deprecation. Keep a feature flag for one release
cycle, then remove.

**Edge cases**: E12: callers that depended on sync response → return task id + document
id; the existing track-status endpoint already supports polling.

**Risk**: LOW. **Estimate**: 0.5 day. **Acceptance**: `POST /upload?async=false` returns
202; the document is processed by the worker.

### P-G3 — Fix Global mode N+1 `node_degree` ✅ (RC-8, HIGH)

> **Status: DONE, WIRED, TESTED (2026-06-26).**
> - `engine_impl/vector_queries.rs` Global arm (Step 5, `:383-399`) now issues a single
>   `graph.node_degrees_batch(&entity_ids)` call via `tokio::join!` with
>   `get_nodes_batch`, mirroring Local mode (`:151`). The per-entity
>   `graph.node_degree(id)` loop is gone.
> - Edge case E13 (node disappears between batch calls) handled via `unwrap_or(0)`.
> - Acceptance verified: `contract_global_no_nplus1.rs` → a Global query over a seeded
>   graph returns entities with degrees populated by the batched path. The structural
>   "exactly one call" property is enforced by code construction (the loop was deleted,
>   not merely bypassed).

**Goal**: Global mode must use `node_degrees_batch` like Local mode.

**Files**: `edgequake/crates/edgequake-query/src/sota_engine/vector_queries.rs:387-393`

**Design**: replace

```387:392:edgequake/crates/edgequake-query/src/sota_engine/vector_queries.rs
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = graph.node_degree(id).await?;
```

with a single `graph.node_degrees_batch(&entity_ids)` call (as Local does at
`vector_queries.rs:149-152`), then look up degrees from the map.

**Edge cases**: E13: a node disappears between `get_nodes_batch` and
`node_degrees_batch` → `unwrap_or(0)`.

**Risk**: LOW. **Estimate**: 0.25 day. **Acceptance**: a Hybrid query over a graph with
E entities performs **exactly one** `node_degrees_batch` call in the Global arm (count
via a storage call counter in tests).

### P-G4 — Batch vector + graph writes ✅ DONE (RC-9, HIGH)

> **Status (2026-06-26, plan-24):** Vectors batched in merger; graph uses
> `get_nodes_batch` + `upsert_nodes_batch` (entities) and
> `get_edges_for_nodes_batch` + `upsert_edges_batch` (relationships).

**Acceptance met (vectors):** one `vector.upsert` call for all entity embeddings in a
document merge batch (not E calls).

**Still open:** batch graph node/edge writes via `upsert_nodes_batch` inside merger;
edge prefetch N+1 in processor is **gone** (deleted with P-G2).

~~**Files** (obsolete — processor manual path removed):~~
- ~~`text_insert.rs:686-714`~~ → deleted
- ~~`text_insert.rs:988-1022`~~ → deleted

**Design**: (subsumed by P-G2 for the persister, but listed separately because it can
ship first as an incremental fix on the processor without the full persister refactor)
collect all `(id, embedding, metadata)` tuples into one `Vec` and call
`workspace_vector_storage.upsert(&all)` once. Track `written_chunk_vector_ids` and
`written_entity_vector_ids` from the `Vec` (still available for compensation).

**Edge cases**: E14: one vector in the batch has wrong dimension → the whole batch
fails; the persister must validate dimensions up front and reject before any write.
E15: provider batch size limit (e.g. 1000) → the postgres adapter already chunks
internally (`storage_impl.rs` UNNEST); confirm and document.

**Risk**: LOW. **Estimate**: 0.5 day. **Acceptance**: ingest a 30-chunk doc; assert
**one** `vector.upsert` call (not 30) via a storage call counter.

### P-G5 — Complete the saga ✅ DONE (new-write scope) (RC-10, HIGH)

> **Status (2026-06-26, re-assessed):** `compensate_merge_failure` in
> `edgequake-storage/src/compensation.rs` rolls back chunk vectors, entity/rel
> vectors recorded in `MergeArtifacts`, and newly created graph nodes/edges.
> Wired from `persist_processing_result` on merge failure. Processor marks persist
> failure as `failed` (not `partial_failure`).

**Honest limit:** compensation targets **this session's new writes** only. Updates
to pre-existing merged nodes are intentionally not rolled back (correct for idempotent
merge). Partial merge with `stats.errors > 0` triggers full session rollback.

~~**Files** (obsolete line refs):~~ processor edge/entity manual paths removed.

**Acceptance:** `compensation::tests::compensate_merge_failure_rolls_back_new_graph_and_vectors` green.

**Design**:
- After edge-batch failure, call `compensate_orphan_vectors(vs, doc_id,
  &written_chunk_vector_ids, &written_entity_vector_ids, &cause)`.
- After entity-vector failures where `entity_embedding_failures > 0`, decide: if
  `> threshold` → compensate and mark `partial_failure`; if `<= threshold` → log and
  continue (current behavior). Threshold configurable, default 0 (any failure →
  partial_failure) to match the merger's strictness.
- Sync upload path: removed by P-G2b, so this concern disappears. If P-G2b is deferred,
  add compensation to the sync path as an interim measure.

**Edge cases**: E16: compensation itself fails → quarantine log (existing). E17:
partial graph write before failure → graph MERGE idempotent (existing). E18: cancel
mid-compensation → check cancel token between steps.

**Risk**: MEDIUM. **Estimate**: 1 day. **Acceptance**: inject an edge-batch failure in
a test; assert chunk + entity vectors are deleted and status is `partial_failure`.

### P-G6 — Delete dead query code; fix API fake rerank ✅ (RC-11, HIGH)

> **Status: DONE, WIRED, E2E tested (2026-06-26).**
> - P-G6a: deleted `strategies/` (incl. `query_bench.rs`), `chunk_retrieval.rs`, and the
>   legacy `QueryEngine` struct. Shared protocol types (`QueryRequest`, `QueryResponse`,
>   `QueryStats`, `ConversationMessage`) hoisted to `crates/edgequake-query/src/types.rs`.
>   `decode_entity_name_from_result` moved to `helpers.rs`. `engine.rs` is now a thin
>   re-export shim. `QueryRuntime.query_engine` field removed (never read by any handler);
>   `build_production_query_engines` → `build_production_query_engine`.
> - P-G6b: `SOTAQueryEngine` → `QueryEngine`, `SOTAQueryConfig` → `QueryEngineConfig`,
>   `sota_engine/` → `engine_impl/` (SOTA means nothing). Dead legacy `QueryEngineConfig`
>   struct removed from `types.rs` (one engine, one config).
> - P-G6c: removed the fake rerank in `query_execute.rs` (no more `(score*0.95+0.05)`
>   mutation, no fabricated `rerank_time_ms = Some(5)`). Added `QueryEngine::has_reranker()`;
>   the handler now reports `reranked = enable_rerank && has_reranker()` truthfully and
>   preserves the engine's rerank ordering.
> - Verified: `cargo build --workspace --tests --examples` (postgres) passes; `cargo test
>   -p edgequake-query` 72 lib + all integration suites green; `cargo test -p edgequake-api
>   --lib` 651 passed; `cargo test -p edgequake-core --lib` 149 passed; `streaming_query`
>   example runs end-to-end; clippy clean on `edgequake-query`.
> - Acceptance check: `rg "use .*::engine::QueryEngine|strategies::|chunk_retrieval"`
>   returns zero hits in `edgequake/`.

**Goal**: one query engine, no dead modules, no fake rerank.

**Files**:
- delete `edgequake/crates/edgequake-query/src/strategies/` (benchmark-only; move any
  still-needed bench into a `benches/` module that uses the SOTA engine)
- delete `edgequake/crates/edgequake-query/src/engine.rs` (legacy `QueryEngine`)
- remove its construction in `query_bootstrap.rs:33-39`
- delete `edgequake/crates/edgequake-query/src/chunk_retrieval.rs` (dead, fake rerank)
- `edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs:229-299`
  (replace fake rerank with the real `Reranker` trait call used by the streaming path;
  if no reranker is configured, set `rerank_time_ms = None` and do not mutate scores)

**Design**: the sync `/query` handler must call the same `rerank_chunks` path as
`pipeline_finalize` (`query_pipeline.rs:319-331`). No score mutation in the API layer.

**Edge cases**: E19: benches that imported `strategies::*` → rewrite against
`SOTAQueryEngine`. E20: `engine.rs` re-exported types (`QueryRequest`, `QueryResponse`,
`QueryStats`) → move them to `sota_engine` or a `types.rs` module so callers don't break.

**Risk**: MEDIUM (deletion; ensure no hidden importers). **Estimate**: 1.5 days.
**Acceptance**: `rg "use .*::engine::QueryEngine|strategies::|chunk_retrieval"` returns
zero hits; `cargo test --workspace` passes; enabling `rerank` on sync `/query` changes
ordering identically to the streaming path on the same fixture.

---

## 4. Phase H — Performance

### P-G7 — Eliminate O(W) KV `keys()` scans ✅ DONE, WIRED, TESTED (RC-12, MEDIUM)

**Status (2026-06-26)**: Every production hot path that previously called
`kv_storage.keys()` and filtered in-memory now uses the index-friendly
`keys_with_prefix` / `keys_with_suffix` trait methods. The Postgres adapter
overrides these with B-tree / `reverse(key)` expression-index scans
(O(log N + K)); other adapters filter in-process. Either way, no caller pays
an O(W) full-table key scan anymore.

**Call sites changed** (verified by grep — zero `.keys().await` remain in the
named targets):

- `edgequake-api/processor/pdf_processing.rs` resume-cleanup:
  `keys()` + `starts_with(chunk_prefix)` → `keys_with_prefix(&chunk_prefix)`.
- `edgequake-api/handlers/documents/recovery/reprocess.rs`:
  `keys()` + `ends_with("-metadata")` → `keys_with_suffix("-metadata")`
  (and the now-redundant suffix filter in the loop was dropped — DRY).
- `edgequake-api/handlers/documents/recovery/stuck.rs`:
  same `keys_with_suffix("-metadata")` conversion.
- `edgequake-api/handlers/documents/storage_helpers.rs` re-convert cleanup:
  `keys()` + per-doc prefix filter → `keys_with_prefix(&doc_prefix)` then a
  tiny in-memory content/chunk subset filter.
- `edgequake-api/handlers/documents/delete/single.rs`:
  `resolve_kv_key_prefix` rewritten to take the `AppState` and do a lazy
  `keys_with_suffix("-metadata")` scan only on the slow mismatch-resolution
  path (fast path is an O(1) `get_by_id`); chunk/content/lineage keys are
  fetched via `keys_with_prefix(&doc_prefix)`. Test call sites updated.

**Design decision (Code is Law)**: the spec proposed a relational
`document_chunks` table (Option 2) or a KV-side chunk-index key (Option 1).
The implemented fix uses the **existing** `keys_with_prefix` / `keys_with_suffix`
trait methods that the Postgres adapter already overrides with index-friendly
scans. This achieves the acceptance criterion (zero full `keys()` scans) with
the smallest, DRY change and no schema migration — the `document_chunks`
relational index remains a tracked future option aligned with plan-17 CQRS.

**Acceptance**: reprocessing/deleting a doc in a workspace with 100k keys
performs zero full `keys()` scans. Verified structurally (grep shows no
`.keys().await` in the named production paths) and by the existing 651 api
lib tests + delete tests remaining green.

**Files**:
- `edgequake/crates/edgequake-api/src/handlers/documents/recovery/reprocess.rs:64-118`
- `edgequake/crates/edgequake-api/src/processor/pdf_processing.rs:287-292`
- `edgequake/crates/edgequake-storage/src/traits/kv.rs` (add a `keys_with_prefix(prefix)`
  or rely on an index)

**Design**: introduce a per-document chunk index. Options:
1. **KV-side prefix index**: maintain a `{doc_id}-chunk-index` key listing all chunk IDs
   written; on reprocess/resume, read one key instead of scanning W.
2. **Relational index**: a `document_chunks` table (chunk_id PK, doc_id FK, index,
   workspace_id) written by the persister (P-G2). Query by `WHERE doc_id = $1`.

Option 2 is preferred (it aligns with the CQRS read-model direction of plan-17 and
gives O(log N) indexed access). Option 1 is a cheaper interim.

**Edge cases**: E21: chunk index out of sync with KV → invariant INV-B (plan-17) detects.
E22: legacy docs without index → fall back to scan once, then build the index.

**Risk**: MEDIUM. **Estimate**: 1.5 days. **Acceptance**: reprocessing a doc in a
workspace with 100k keys performs **zero** full `keys()` scans; measured via a KV call
counter.

### P-G8 — Fix Bypass mode; implement real Mix mode ✅ DONE (RC-13, MEDIUM)

> **Status (2026-06-26, plan-24):** Engine + HTTP contracts green.
> `contract_query_modes.rs` (3/3); `e2e_spec021_query_modes_http.rs`.

**Design (Mix)**: expose `mix_local_weight`, `mix_global_weight`, `mix_naive_weight`
in `SOTAQueryConfig`; Mix mode runs the three arms (reusing Hybrid's `tokio::join!`) and
applies weighted score normalization (e.g. min-max per arm then weighted sum) instead
of round-robin. If all weights equal, behavior matches Hybrid (backward compatible).

**Edge cases**: E23: Bypass with empty query → return direct LLM answer. E24: Mix
weights sum to 0 → fall back to equal weights + log warning. E25: an arm returns 0
results → its contribution is 0, not an error.

**Risk**: LOW. **Estimate**: 1 day. **Acceptance**: `POST /query {mode:"bypass"}`
returns a direct LLM answer (not the apology string); `mode:"mix"` with
`{local:0.5,global:0.5,naive:0.0}` produces ordering different from `mode:"hybrid"` on
a fixture where local and global disagree.

### P-G9 — Query-result and query-embedding caches ✅ DONE (RC-14, MEDIUM)

**Status (2026-06-26, plan-24):** Both halves wired in production bootstrap.

**Embedding cache (DONE)**:
- new `edgequake/crates/edgequake-query/src/cache/embedding_cache.rs`
  (`CachingEmbeddingProvider` wraps any `EmbeddingProvider` and memoizes
  `embed_one` keyed by `hash(model + text)`, LRU 10k entries, 1h TTL).
- DIP/DRY: the cache is a transparent decorator — `embed` (batch, ingestion) is
  delegated unchanged so ingestion semantics are untouched; only the query-path
  `embed_one` is cached.
- E27 (embedding model change): the `{name}/{model}` identity is folded into the
  cache key, so a model swap invalidates everything without a manual clear.
- Wired into the production query engine at the single DRY construction point
  `edgequake-api/src/state/query_bootstrap.rs::build_production_query_engine`
  via `QueryEngine::with_embedding_cache()` (new builder on `QueryEngine`).
- Contract tests: `edgequake-query/tests/contract_embedding_cache.rs`
  (repeated identical query → inner provider called once, 2 cache hits;
  batch embed bypasses the cache; distinct queries each miss) + lib unit tests
  (LRU, TTL expiration). All green.

**Query-result cache (DONE)**:
- `edgequake-query/src/cache/query_result_cache.rs` — LRU + TTL, `bump_epoch()` invalidation
- Wired via `QueryEngine::with_result_cache()` in `query_bootstrap.rs`
- Contract: `contract_query_result_cache.rs`

~~**Query-result cache (DEFERRED)**~~: superseded by plan-24.

**Goal**: stop re-embedding identical queries and recomputing identical retrieval.

**Files**:
- `edgequake/crates/edgequake-query/src/sota_engine/mod.rs` (add an
  `embedding_cache: Arc<dyn EmbeddingCache>` keyed by `hash(query)`)
- new `edgequake/crates/edgequake-query/src/cache/query_cache.rs`
  (`QueryResultCache` keyed by `hash(query + mode + tenant + workspace + filter)`,
  TTL configurable, default 5 min, invalidated on ingestion to the same workspace)
- `edgequake/crates/edgequake-api/src/cache_manager.rs` (wire)

**Design**:
- Embedding cache: `hash(query) -> Vec<f32>`. Hits skip the `embed_one` call. LRU,
  10k entries, 1h TTL.
- Result cache: only for `context_only` and non-streaming queries (never cache
  generated answers, which should reflect conversation history). Cache the
  `QueryContext` + `QueryStats` for `context_only`; for full queries cache nothing
  (generation is the expensive, non-deterministic part).
- Invalidation: ingestion to workspace W bumps a workspace `ingestion_epoch`; cache
  entries stamped with an older epoch are evicted on read.

**Edge cases**: E26: cache stampede on popular query → single-flight via
`tokio::sync::Dedup`. E27: embedding model change → bump a global `embedding_version`
key, evicting all. E28: filtered query → include filter hash in key.

**Risk**: MEDIUM (correctness of invalidation). **Estimate**: 2 days. **Acceptance**:
two identical `context_only` queries perform one `embed_one` and one retrieval round;
the second hits both caches (assert via call counters). Ingesting a doc evicts the
workspace's cached contexts.

---

## 5. Phase I — Hygiene & Contracts

### P-G10 — Make batch trait methods required ✅ DONE (RC-15, MEDIUM)

> **Status (2026-06-26):** `graph_mutate_ops.rs` documents required
> `upsert_nodes_batch` / `upsert_edges_batch` with no default loop impl (P-G10 / RC-15
> comment in trait). Memory adapter implements real batch.

**Edge cases**: E29: third-party trait impls → they must implement the method; this is
the intended breaking change.

**Risk**: LOW. **Estimate**: 0.5 day. **Acceptance**: memory adapter `upsert_nodes_batch`
on 100 nodes performs one internal mutation, not 100 (assert via a counter).

### P-G11 — Streaming backpressure + vision parity ✅ DONE, WIRED, TESTED (RC-16, LOW)

**Status (2026-06-26)**: Both halves landed.

- **Vision parity**: `engine_impl/query_entry/query_stream.rs::stream_answer_from_context`
  now checks `request.images` *before* the empty-context apology and delegates to
  the new `engine_impl/prompt.rs::stream_vision_answer`, which builds the vision
  chat messages (`build_vision_system_message` + `ChatMessage::user_with_images`)
  and calls `provider.chat()`. The `LLMProvider::stream` trait method cannot
  carry images, so the vision path uses `chat` and emits the result as a
  one-shot token stream — the same trade-off the sync path already makes.
  E30 (vision LLM unavailable): `stream_vision_answer` falls back to the
  text-only `stream`/`complete` path on `chat` failure, mirroring
  `generate_answer_with_provider`'s image fallback.
- **Backpressure**: the API streaming handler uses a bounded `mpsc::channel(100)`
  and the token loop is strictly sequential — `stream.next().await` then
  `tx.send(event).await`. Because `send().await` suspends when the channel is
  full, the LLM read is paused automatically (no buffering between read and
  send). Backpressure is therefore structural: channel depth can never exceed
  100, satisfying the E31 acceptance ("slow consumer → no unbounded LLM
  buffering, no token loss"). No `StreamFlushManager` was needed for the query
  path (that helper is for the document pipeline's coalesced writes).

**Contract tests**: `edgequake-query/tests/contract_streaming_vision.rs`
(`streaming_query_with_images_uses_vision_chat_path` — asserts `chat()` is
called once with images and `stream()` is NOT called;
`streaming_query_without_images_uses_text_stream_path` — asserts `stream()` is
used and `chat()` is not). Both green.

**Goal**: streaming path matches sync path's vision handling and applies backpressure.

**Files**:
- `edgequake/crates/edgequake-query/src/sota_engine/query_entry/query_stream.rs:83-132`
  (pass `request.images` into the prompt builder; use the vision-capable LLM path)
- `edgequake/crates/edgequake-api/src/handlers/query/query_stream.rs:335-341`
  (bounded channel already 100; add `StreamFlushManager` usage for backpressure —
  pause LLM read when channel is >80% full)

**Edge cases**: E30: vision LLM unavailable → fall back to text (existing in
`prompt.rs:227-234`). E31: client slow → backpressure pauses generation, no token loss.

**Risk**: LOW. **Estimate**: 1 day. **Acceptance**: a streaming query with `images`
uses the vision prompt; a slow consumer (sleep 100ms per token) does not cause
unbounded LLM buffering (assert channel depth stays ≤100).

### P-G12 — Workspace-scoped analytics defaults + contract tests ✅ DONE (RC-17, LOW)

> **Status (2026-06-26, re-assessed):** `node_count_by_workspace` / `edge_count_by_workspace`
> are **required** trait methods (no workspace-ignoring default). Postgres adapter filters
> by `workspace_id` in Cypher. Contract: `edgequake-storage/tests/contract_workspace_scoped_analytics.rs`
> (E32 empty workspace → 0). Query/ingestion contracts live in their respective crates as listed below.

**Files**:
- `edgequake/crates/edgequake-storage/src/traits/graph_analytics_ops.rs:30-37`
  (remove the workspace-ignoring default; make `node_count_by_workspace` required, or
  provide a default that actually filters)
- new `edgequake/crates/edgequake-core/tests/contract_ingestion_persistence.rs`
  (P-G2 acceptance: three callers → identical storage state)
- new `edgequake/crates/edgequake-core/tests/contract_entity_identity.rs`
  (P-G1 acceptance: casing variants → one node + one vector)
- new `edgequake/crates/edgequake-query/tests/contract_query_modes.rs`
  (P-G8 acceptance: Bypass returns direct LLM; Mix ≠ Hybrid on a divergent fixture)
- new `edgequake/crates/edgequake-query/tests/contract_global_no_nplus1.rs`
  (P-G3 acceptance: one `node_degrees_batch` call in Global arm)

**Edge cases**: E32: a workspace with zero nodes → `node_count_by_workspace` returns 0
(not the global count).

**Risk**: LOW. **Estimate**: 1.5 days. **Acceptance**: all contract tests green;
`node_count_by_workspace(W_A)` does not count nodes belonging to `W_B`.

### P-G13 — Interactive availability under ingestion load ✅ (RC-18, HIGH)

> **Status: DONE, WIRED, TESTED (2026-06-26).**
> Terminal proof during 3 concurrent PDF jobs: `/health` 200 in 2–4ms while workers
> 9/13/14 ran Mistral vision extraction — backend was alive; banner was a false
> negative from probing deep health with a 2s timeout and no retry.

**Goal**: stop the dashboard from declaring the backend "not reachable" during heavy
ingestion when the process is alive but the DB pool is busy.

**First principles**:
- **Liveness ≠ readiness ≠ deep health.** `/live` (process up, zero DB) → banner gate.
  `/health` (storage pings) → `degraded` status only.
- **Stale-if-error beats zero.** Workspace stats must return last-known counts with
  `stale: true` rather than timing out → React Query shows 0.
- **Bounded probes.** Storage `ping()` in `/health` gets 750ms each (parallel); never
  block on `acquire_timeout` (5s).

**Files**:
- new `edgequake/crates/edgequake-api/src/handlers/health_probes.rs` (DRY bounded pings)
- `edgequake/crates/edgequake-api/src/handlers/health.rs` (use probes; storage failure → `degraded`)
- `edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs` (4s timeout + stale-if-error)
- `edgequake/crates/edgequake-api/src/handlers/workspaces_types/responses.rs` (`stale?: bool`)
- `edgequake_webui/src/lib/api/backend-readiness.ts` (`/live` + retries; `degraded` ≠ unreachable)
- `edgequake_webui/src/components/shared/backend-status-banner.tsx` (busy vs down copy)

**Edge cases**:
- E33: `/live` ok, `/health` timeout → `degraded`, banner says "busy processing".
- E34: stats cache miss + fetch timeout → 503 unless stale cache exists.
- E35: cold start `/live` fails → 3 retries then `unreachable`.

**Risk**: LOW. **Estimate**: 0.5 day. **Acceptance**: upload 3 PDFs concurrently;
Documents page never shows "backend not reachable" while `/live` returns OK; stats
never flip to 0 when stale cache exists.

### P-G14 — Idempotent PDF ingest admission under tenant pressure ✅ (RC-19, HIGH)

> **Status: DONE, WIRED, TESTED (2026-06-26).**
> Symptom: same filename appears 3–4× in Documents (Pending + Completed) after heavy
> ingestion when `MAX_TASKS_PER_TENANT` pauses work — not user re-upload alone.

**Goal**: one logical document per PDF row; retries/requeues/orphan recovery must
never mint a second `document_id`.

**First principles**:
- **Identity precedes side effects**: allocate `document_id` at enqueue; persist on
  the task row *before* KV metadata writes.
- **Single-flight per pdf_id**: while PdfProcessing is pending/processing, return the
  existing `track_id` instead of enqueueing again (unless `restart_from_scratch`).
- **Ground-truth resume**: worker checks KV metadata existence, not merely
  `task_data.existing_document_id`, to decide resume vs fresh conversion.

**Files**:
- new `edgequake-api/src/services/ingest_admission.rs` (SSOT resolver + single-flight)
- `edgequake-api/src/services/pdf_workspace_dedup.rs` (DRY `find_kv_document_id_for_pdf`)
- `edgequake-api/src/handlers/pdf_upload/helpers.rs` (enqueue-time id + admission)
- `edgequake-api/src/processor/pdf_processing.rs` (persist id before metadata)
- `edgequake-api/src/processor/mod.rs` (`with_task_storage`)
- `edgequake_webui/src/hooks/use-file-upload.ts` (optimistic row uses `document_id`)

**Edge cases**:
- E36: backend OOM after metadata write but before task persist → worker resolves id
  from KV `pdf_id` index, not a new UUID.
- E37: tenant limiter requeues same task → no second document (processing not started).
- E38: concurrent upload while task pending → single-flight returns existing track_id.
- E39: `force_reindex` / `restart_from_scratch` bypasses single-flight intentionally.
- E40: under `MAX_TASKS_PER_TENANT`, extra PDFs get KV shell with `status=queued` at enqueue so
  the Documents list shows all uploads immediately (not only when a worker slot opens).

**Risk**: LOW. **Estimate**: 1 day. **Acceptance**: upload 3 PDFs under
`MAX_TASKS_PER_TENANT=2`; Documents list shows exactly 3 rows; retry/orphan restart
does not increase row count for the same `pdf_id`.

---

## 6. Prioritized execution order (updated 2026-06-26)

**Done (correctness + perf + SOLID):** P-G1, P-G2/P-G2b/P-G2d, P-G3, P-G4, P-G5, P-G6, P-G7, P-G8, P-G9, P-G10, P-G11, P-G12, P-G13, P-G14, P-G15–G18.

**Remaining (accepted):**

1. **P-G1b** — legacy entity backfill (admin-gated; destructive)
2. **Full 8-step persister** — KV/relational/lineage in processor (scope decision)
3. **Postgres UNWIND worker E2E** — optional; covered by adapter tests

~~Original 18-day schedule below is historical; do not use for planning.~~

---

## 7. Edge-case registry (consolidated for this plan)

| ID | Edge case | Handled by |
|----|-----------|------------|
| E1 | Empty entity name | P-G1 skip + warn |
| E2 | `entity:` prefix already present | P-G1 strip before normalize |
| E3 | Non-ASCII names | P-G1 test |
| E4 | Legacy un-normalized data | P-G1b backfill |
| E5 | Two raw variants → same key | P-G1b merge |
| E6 | Node already normalized | P-G1b skip |
| E7 | Backfill race with ingestion | P-G1b skip processing |
| E8 | Postgres feature disabled | P-G2 cfg gate |
| E9 | Checkpoint resume | P-G2 idempotent persist |
| E10 | Sync upload callers | P-G2b 202 + task id |
| E11 | Per-entity LLM summarization | P-G2 optional summarizer hook |
| E12 | Sync-dependent callers | P-G2b track-status polling |
| E13 | Node disappears between batch calls | P-G3 unwrap_or(0) |
| E14 | Wrong-dimension vector in batch | P-G4 pre-validate |
| E15 | Provider batch-size limit | P-G4 adapter chunks internally |
| E16 | Compensation fails | P-G5 quarantine log |
| E17 | Partial graph write | P-G5 idempotent MERGE |
| E18 | Cancel mid-compensation | P-G5 cancel token |
| E19 | Benchs importing strategies | P-G6 rewrite vs SOTA |
| E20 | engine.rs re-exported types | P-G6 move to sota_engine/types |
| E21 | Chunk index out of sync | P-G7 INV-B detects |
| E22 | Legacy docs without index | P-G7 one-time scan + build |
| E23 | Bypass with empty query | P-G8 direct LLM |
| E24 | Mix weights sum to 0 | P-G8 equal-weight fallback |
| E25 | An arm returns 0 results | P-G8 zero contribution |
| E26 | Cache stampede | P-G9 single-flight |
| E27 | Embedding model change | P-G9 version key |
| E28 | Filtered query caching | P-G9 filter hash in key |
| E29 | Third-party trait impls | P-G10 breaking change |
| E30 | Vision LLM unavailable | P-G11 text fallback |
| E31 | Slow consumer | P-G11 backpressure |
| E32 | Workspace with zero nodes | P-G12 returns 0 |
| E33 | Live ok, health timeout under load | P-G13 degraded banner |
| E34 | Stats fetch timeout with stale cache | P-G13 serve stale |
| E35 | Cold start live probe fails | P-G13 retry then unreachable |

---

## 8. What this plan does NOT do (and why)

| Not doing | Why |
|-----------|-----|
| Implement LightRAG community detection | Out of scope for this plan; tracked as a future "Global v2" initiative. Document the gap honestly in `modes.rs` instead (P-G8 doc fix). |
| Replace pgvector with a dedicated vector DB | pgvector HNSW is adequate for the foreseeable corpus scale; the bottleneck is round-trips (P-G4) and caching (P-G9), not ANN quality. |
| Synchronous 2PC across stores | No coordinator; saga + invariant remains correct (plan-17 §11). |
| Auto-run the legacy entity backfill (P-G1b) | Destructive merge; admin-gated with dry-run + confirm token. |
| Cache generated LLM answers | Non-deterministic and conversation-dependent; only `context_only` retrieval is cached (P-G9). |

---

## 9. Success metrics (re-assessed 2026-06-26)

| Metric | Before | Target (original plan) | **Actual (Code is Law)** |
|--------|--------|------------------------|---------------------------|
| Duplicate nodes for one entity (new writes) | possible (RC-6) | 0 | ✅ 0 (`EntityId` + merger) |
| Legacy duplicate nodes | possible | 0 after P-G1b | ⚠️ until admin reconcile |
| Ingestion persist divergence (orchestrator vs processor) | 3 paths | 1 trait, 3 callers | ✅ 1 function, 2 callers (+ enqueue-only upload) |
| Chunk vector upserts per doc | O(chunks) | 1 | ✅ 1 batched |
| Entity vector upserts per doc | O(E) | 1 | ✅ 1 batched (merger) |
| Graph node upserts per doc | O(E) | 1 batch | ✅ batched in merger |
| Global `node_degree` calls | E | 1 | ✅ 1 (`node_degrees_batch`) |
| Orphan vectors on merge failure (new writes) | possible | 0 | ✅ saga extended |
| Query engines | 3 | 1 | ✅ 1 |
| KV full `keys()` scans on reprocess | 1+ | 0 | ✅ 0 (prefix/suffix) |
| Repeated-query embedding calls | every request | 1 (cache) | ✅ cached |
| Repeated `context_only` retrieval | every request | cached (P-G9) | ✅ `QueryResultCache` |
| Bypass / Mix at engine | broken / fake | fixed | ✅ engine + contracts |
| Bypass / Mix at HTTP | — | fixed | ✅ HTTP E2E |
| Dashboard workspace node counts | cross-tenant leak | scoped | ✅ contract + Postgres filter |

---

## 10. Task logs (historical — authorship of plan-19)

> Superseded by §13 for current status. Kept for traceability only.

Actions: Authored file 18 audit → mapped RC-6..RC-19 to P-G1..G14 with acceptance tests.

Decisions: Correctness-first ordering; newtype identity; async-only upload; delete dead query engines.

Next steps (at authoring time): P-G1 → P-G3 → P-G4/P-G5 → P-G6 → P-G2. **All executed** except
P-G1b (admin), P-G9-result, P-G4-graph, P-G8-http, P-G2d trait.

Lessons: Abstraction-inversion (compute clean, persistence ad-hoc) was the root cause. Partial
fix shipped as free-function persister — good enough for RC-7, not full SOLID story.

---

## 11. Multi-perspective assessment (2026-06-26, pre-commit verification)

Reviewed the implemented changes (P-G1, P-G3, P-G6, P-G2b) against four lenses:
**GraphRAG**, **LightRAG**, **AI Engineer**, and **System Engineer**, using
"Code is Law" + First Principles (the code as written is the ground truth, not
the docs).

### 11.1 GraphRAG lens

- **Identity (RC-6 fix, P-G1):** Correct and idiomatic. `EntityId` is a newtype;
  graph node id and entity vector id are *derived* from one value, so the
  LightRAG-style fragmentation (same entity → multiple nodes / invisible entity
  vectors) cannot recur by construction. This matches the LightRAG invariant that
  the entity name is the single join key across the vector store and the graph.
- **Global mode (RC-8 fix, P-G3):** The batched `node_degrees_batch` restores the
  LightRAG "global context = high-degree entities" semantics without the N+1
  that made Global mode quadratically expensive. Degrees are now fetched in one
  round-trip, matching Local mode's already-correct path.
- **Gap vs LightRAG/GraphRAG SOTA:** Community detection / hierarchical
  summarization is still absent (acknowledged in §8). Global mode is "flat"
  LightRAG, not GraphRAG. This is an honest, documented limitation — not a
  regression.

### 11.2 LightRAG lens

- The merge logic (`merger/entity.rs`) preserves LightRAG parity: description
  merge, importance max, `source_chunk_ids` union for citation tracking, optional
  LLM summarization hook. `EntityId` is threaded through without disturbing these.
- One LightRAG-correctness win: the entity vector metadata's `entity_name` is now
  the *normalized* name (verified by `contract_entity_identity.rs`), so the query
  decoder recovers exactly the graph node id — closing the silent "entity vector
  written but never retrieved" bug from file 18.

### 11.3 AI Engineer lens

- **Dead code removal (P-G6):** `strategies/`, `chunk_retrieval.rs`, the legacy
  `QueryEngine`, and the fake rerank (`score*0.95+0.05` + fabricated
  `rerank_time_ms=Some(5)`) are gone. One engine, one config, truthful
  `has_reranker()` reporting. This removes a real AI-engineering hazard: a fake
  rerank that *reordered* results while claiming to improve them.
- **Sync upload (P-G2b):** Forcing `202 ACCEPTED` + `task_id` is the correct
  contract for an extraction pipeline that calls an LLM (seconds-to-minutes).
  Inline sync persistence with no saga was a correctness and UX liability.

### 11.4 System Engineer lens — **regression found and fixed**

- **Regression (caught by `cargo test -p edgequake-api --lib --no-default-features`):**
  the new empty-markdown fallback in `reprocess.rs` referenced
  `state.storage.pdf_storage` unconditionally, but `StorageRuntime::pdf_storage`
  is `#[cfg(feature = "postgres")]`-gated. The crate compiled under the default
  CI invocation (which enables `postgres`) but **failed to compile without the
  feature** — a feature-gating hole. The pre-existing pattern at `reprocess.rs:444`
  gates the same access with `#[cfg(feature = "postgres")]`.
- **Fix applied:** added `#[cfg(feature = "postgres")]` to the new block and
  `#[allow(unused_mut)]` on the two bindings that are only mutated under that
  feature. Now compiles cleanly both with and without `postgres`.
- **Lesson:** "Code is Law" cuts both ways — the build matrix that isn't run is a
  blind spot. The lib-only-without-default-features build must stay green because
  it is the cheapest static check that feature gates are honest. Recommend adding
  it to CI.

### 11.5 Verification matrix (this commit)

| Check | Command | Result |
|-------|---------|--------|
| Build (all features) | `cargo build --workspace --tests --examples` | ✅ clean |
| Build (api, no postgres) | `cargo build -p edgequake-api --lib --no-default-features` | ✅ clean (after fix) |
| EntityId unit tests | `cargo test -p edgequake-storage --lib entity_id::` | ✅ 9/9 |
| P-G1 contract | `cargo test -p edgequake-pipeline --test contract_entity_identity` | ✅ 2/2 |
| P-G3 contract | `cargo test -p edgequake-query --test contract_global_no_nplus1` | ✅ 1/1 |
| api lib (postgres) | `cargo test -p edgequake-api --lib --features postgres` | ✅ 651/651 |
| api lib (no postgres) | `cargo test -p edgequake-api --lib --no-default-features` | ✅ 633/633 |
| query lib + integration | `cargo test -p edgequake-query --lib --tests` | ✅ all green |
| core lib | `cargo test -p edgequake-core --lib` | ✅ 135/135 |
| clippy (touched crates) | `cargo clippy -p edgequake-api -p edgequake-query -p edgequake-storage -p edgequake-pipeline --lib --features postgres` | ✅ no new warnings |

### 11.6 Remaining plan-19 items — **STALE; see §13 (2026-06-26 re-assessment)**

~~P-G2 open~~ → **closed** (plan-23). ~~P-G4/P-G5 open on processor~~ → **closed on
merger path**. ~~P-G8 open~~ → **engine closed**. ~~P-G12~~ → **closed**. Open: P-G1b,
P-G9-result, P-G4-graph batch, P-G8-http contract.

---

## 12. Multi-perspective assessment — P-G13 + P-G14 (2026-06-26, pre-commit)

Reviewed the operational-resilience changeset (availability under load + PDF
ingest admission) against four lenses: **GraphRAG**, **LightRAG**, **AI Engineer**,
**System Engineer**. Method: "Code is Law" — claims below cite what the code
*actually does*, not what we wish it did.

### 12.1 GraphRAG lens — **neutral / indirect benefit**

- **No graph algorithm change.** P-G13/P-G14 do not touch retrieval, community
  detection, or hierarchical summarization. GraphRAG SOTA gap (flat Global mode,
  no Leiden/Louvain communities) remains exactly as documented in §8.
- **Indirect win:** fewer duplicate document rows (P-G14) means fewer spurious
  chunk/entity extraction runs against the same PDF bytes — less graph pollution
  from redundant ingestion attempts. This is hygiene, not intelligence.
- **Brutal truth:** if the user uploads 3 PDFs and the UI no longer lies about
  backend health, they still get a **flat** LightRAG graph with no community
  summaries. Operational polish ≠ GraphRAG maturity.

### 12.2 LightRAG lens — **correct identity semantics preserved**

- P-G14 respects the LightRAG invariant that **one document → one extraction
  pipeline → one set of chunks/entities**. The old bug minted multiple
  `document_id`s for one `pdf_id`, which could produce parallel extractions and
  split entity provenance across rows with the same filename.
- `resolve_worker_pdf_document_id` checks KV metadata existence (not just
  `task_data.existing_document_id`) before deciding resume vs fresh conversion —
  this matches LightRAG's "ground truth is what's stored" principle.
- `provision_queued_pdf_document_shell` writes `status=queued` at enqueue so the
  Documents list reflects **all** uploads under `MAX_TASKS_PER_TENANT` pressure,
  not only those that won a worker slot. LightRAG UX expects visible pipeline state.
- **Gap:** single-flight is enforced by **scanning task lists** (`find_active_pdf_processing_task`
  paginates Pending+Processing tasks). There is no DB index on `task_data->>'pdf_id'`.
  At scale this becomes O(tasks) per upload — acceptable for dev, wrong for production
  tenant with thousands of in-flight jobs.

### 12.3 AI Engineer lens — **good UX contracts, missing E2E proof**

- **P-G13 readiness model is correct AI-ops hygiene:** liveness (`/live`) ≠ deep
  health (`/health`). Treating pool saturation as "down" caused operators (and the
  UI) to misdiagnose LLM-heavy ingestion as infrastructure failure. The new
  `BackendReadinessState` enum (`ready | degraded | unreachable | misconfigured`)
  is the right abstraction for a RAG stack where DB and LLM latency decouple.
- **Vision path admission:** `PdfVisionSemaphore` + cloud page concurrency cap
  (8→2) is a **deliberate throughput regression** to stop OOM during multi-PDF
  Mistral vision runs. Correct for stability; wrong if marketed as "fast ingestion."
- **Missing acceptance tests:** plan-19 §5 P-G13/P-G14 acceptance criteria
  ("upload 3 PDFs concurrently; never show unreachable; exactly 3 rows") have
  **no automated E2E test** in this changeset. Unit tests cover admission helpers
  and probe timeouts only. An AI engineer cannot sign off without Playwright or
  integration proof.
- **Stale stats (`stale: true`):** honest degradation, but the UI does not yet
  surface the stale flag to users — counts may look authoritative when they are
  minutes old. Minor honesty gap.

### 12.4 System Engineer lens — **fixes real incidents, leaves sharp edges**

**What landed (verified):**

| Component | Mechanism | Verdict |
|-----------|-----------|---------|
| `health_probes.rs` | 750ms parallel `ping()` per store | ✅ Stops 5s pool acquire blocking `/health` |
| `workspaces/stats.rs` | 4s timeout + stale-if-error | ✅ Prevents React Query 0-flip on slow counts |
| `backend-readiness.ts` | `/live` first, 3 retries, 8s health budget | ✅ Eliminates false unreachable during ingestion |
| `ingest_admission.rs` | SSOT document_id + single-flight + queued shell | ✅ Closes RC-19 root cause |
| `PdfVisionSemaphore` | process-wide cap via `EDGEQUAKE_PDF_VISION_JOBS` | ✅ OOM guard |
| `Makefile` | exports stability defaults | ✅ Dev ergonomics |

**Sharp edges (brutal):**

1. **Single-flight is not transactional.** Two concurrent HTTP uploads of the same
   `pdf_id` can both pass `find_active_pdf_processing_task` before either task row
   exists — classic TOCTOU. Mitigation today: dedup at PDF row level upstream;
   not a serializable lock. Production needs `UNIQUE (pdf_id) WHERE status IN (...)`
   or advisory lock, not list scans.

2. **`find_active_pdf_processing_task` does not scale.** Paginated full-table scan
   of Pending+Processing PdfProcessing tasks per enqueue. Fine for `MAX_TASKS_PER_TENANT=2`;
   breaks at 10k queued tasks.

3. **Cloud PDF concurrency 8→2 is a 4× vision latency hit** for small PDFs on
   OpenAI/Mistral. Necessary given observed OOM; document clearly in ops runbooks.
   `EDGEQUAKE_PDF_CONCURRENCY` override exists but defaults are now conservative.

4. **`/health` can still lie "healthy"** when only 1/3 storage pings succeed within
   750ms — code marks `degraded` only when *all* fail. Partial failure mode is
   under-specified.

5. **No CI matrix entry** for `cargo test -p edgequake-api --lib` on new modules +
   `bun test backend-readiness.test.ts` together. Recommend adding to Makefile `test-spec021`.

6. **Feature-gate discipline:** `resolve_pdf_ingest_document_id` correctly gates
   `pdf_storage` lookup with `#[cfg(feature = "postgres")]` — learned from §11.4
   regression on `reprocess.rs`.

### 12.5 Verification matrix (this commit)

| Check | Command | Result |
|-------|---------|--------|
| ingest_admission unit tests | `cargo test -p edgequake-api --lib ingest_admission` | ✅ 2/2 |
| health_probes unit tests | `cargo test -p edgequake-api --lib health_probes` | ✅ 2/2 |
| backend-readiness TS tests | `bun test src/lib/api/__tests__/backend-readiness.test.ts` | ✅ 6/6 |
| api lib (postgres) | `cargo test -p edgequake-api --lib --features postgres` | ✅ 655/655 |
| clippy touched crates | `cargo clippy -p edgequake-api -p edgequake-core --lib --features postgres` | ✅ no new errors (7 pre-existing needless_borrow) |

### 12.6 Verdict summary

| Lens | Grade | One-line verdict |
|------|-------|------------------|
| GraphRAG | C+ | Ops fix only; no graph intelligence improvement |
| LightRAG | B+ | Restores document identity invariant; task scan won't scale |
| AI Engineer | B | Correct readiness semantics; needs E2E acceptance tests |
| System Engineer | B+ | Closes real production incidents; TOCTOU + throughput trade-offs remain |

**Code is Law conclusion:** P-G13 and P-G14 are **shippable**. E2E gaps (P-G15–G18)
since addressed in plan-20. ~~Next highest leverage P-G2~~ → **done**; see §13 for
what actually remains.

---

## 13. Brutal four-lens re-assessment (2026-06-26, post P-G2 closure)

> **Method:** Code is Law — grep, contract tests, `make test-spec021`. This section is
> the **authoritative** verdict on plan-19 execution. §11–§12 are historical snapshots
> from mid-flight commits; do not treat their "remaining work" lists as current.

### 13.1 Executive verdict

| Dimension | Grade | Honest summary |
|-----------|-------|----------------|
| **Correctness (new writes)** | **A−** | RC-6/7/8/10/11 structurally fixed for the happy path + merge failure |
| **Correctness (legacy data)** | **C** | Pre-G1 graphs still corrupt until admin P-G1b; not auto-healed |
| **Performance** | **B−** | Vector batching + Global N+1 + embedding cache done; graph merge still O(E) |
| **Architecture (SOLID story)** | **C+** | Free-function persister closes DRY; trait/OCP/DIP target in §2.2 **not shipped** |
| **Test honesty** | **B** | Strong contract coverage; worker E2E weak under mock; no HTTP Bypass/Mix proof |
| **GraphRAG / LightRAG maturity** | **C+** | Flat LightRAG; ops fixes ≠ community detection or hierarchical summaries |
| **Plan document quality** | **B−** | Good audit→work-item map; oversold acceptance criteria until this re-assessment |

**Bottom line:** Plan-19 **did its job** — it stopped silent corruption and removed the
worst query/ingestion foot-guns. It **did not** deliver the aspirational "one trait,
eight stores, three byte-identical callers" architecture. Calling P-G2 "DONE" is
**functionally true** (RC-7 closed) and **architecturally false** (§2.2 deferred).

### 13.2 What actually shipped (verified)

| RC | Item | Shipped? | Evidence |
|----|------|----------|----------|
| RC-6 | Entity identity | ✅ new writes | `EntityId`, `contract_entity_identity.rs` |
| RC-7 | Single persist path | ✅ partial | `persist_processing_result`; 2 callers; chunk+merge only |
| RC-8 | Global N+1 | ✅ | `node_degrees_batch`, contract test |
| RC-9 | Vector batching | ✅ vectors / ❌ graph | Merger batch upsert; `get_node`+`upsert_node` per entity |
| RC-10 | Saga | ✅ new-write scope | `compensate_merge_failure` + `MergeArtifacts` |
| RC-11 | Dead query engines | ✅ | P-G6 deletion + real rerank |
| RC-12 | KV `keys()` scans | ✅ | `keys_with_prefix` / `keys_with_suffix` |
| RC-13 | Mix/Bypass | ✅ engine / ⚠️ HTTP | `contract_query_modes.rs`; no API-level test |
| RC-14 | Caching | ◑ | Embedding cache yes; result cache no |
| RC-15 | Batch traits required | ✅ | `graph_mutate_ops.rs` |
| RC-16 | Streaming vision | ✅ | `contract_streaming_vision.rs` |
| RC-17 | Analytics scoping | ✅ | `contract_workspace_scoped_analytics.rs` |
| RC-18 | Availability under load | ✅ | P-G13 + plan-20 E2E |
| RC-19 | PDF idempotency | ✅ best-effort | P-G14; TOCTOU caveats remain |

### 13.3 GraphRAG lens — **C+**

**Wins:** Entity vectors and graph nodes now share one normalized key for new data — retrieval
can join vector hits to graph nodes without the silent "entity written but never found" bug.

**Losses:** Still **flat LightRAG**, not GraphRAG. No Leiden/Louvain communities, no
hierarchical community summaries, no global→local routing beyond degree-weighted entity pick.
P-G13/P-G14 improve **operational** graph hygiene (fewer duplicate extractions) but add
zero retrieval intelligence.

**Brutal truth:** A user who fixed duplicate PDF rows and false "backend down" banners still
queries the same undifferentiated entity soup. Plan-19 never promised GraphRAG SOTA — but
do not confuse "we fixed ingestion corruption" with "we improved RAG quality."

### 13.4 LightRAG lens — **B**

**Wins:**
- Merger remains the canonical merge semantics (description union, `source_chunk_ids`,
  importance max) — now actually on the production path via persister.
- Document identity invariant restored (P-G14): one PDF → one pipeline → one chunk/entity set.
- Async-only upload matches LightRAG's long-running extraction model.

**Losses:**
- **Legacy graphs** may still have casing-duplicated nodes until P-G1b admin job.
- Merger graph writes are **sequential per entity** — LightRAG at scale assumes batched graph
  I/O; we batch vectors but not AGE UNWIND nodes yet.
- Single-flight PDF admission uses **task list scans**, not indexed locks — LightRAG-correct
  semantics, production-wrong scaling.

### 13.5 AI Engineer lens — **B−**

**Wins:**
- Fake rerank removed — sync `/query` no longer lies about reranking.
- Bypass and Mix modes work at the engine with contract tests.
- Embedding cache removes redundant `embed_one` on repeated queries (big win for chat UX).
- Readiness model (`ready | degraded | unreachable`) stops false negatives during vision jobs.

**Losses:**
- **No HTTP contract** for Bypass/Mix — handler could regress without CI catching it.
- **No query-result cache** — repeated identical `context_only` queries still re-run full
  retrieval (embedding half of P-G9 only).
- Worker E2E (`e2e_spec021_ingestion_persister.rs`) often ends in `partial_failure` under
  mock LLM; graph assertions are **gated on success** — CI proves chunks persist, not that
  entity extraction reliably completes in test env.
- Plan-19 originally claimed "byte-identical storage across three callers" — **never tested,
  never true**. Overstated acceptance erodes trust in the plan itself.

### 13.6 System Engineer lens — **B+**

**Wins:**
- RC-7 structural divergence eliminated for chunk vectors + merge — the highest-severity
  correctness bug from file 18.
- Extended saga on merge failure; persist failure → `failed` status (not silent partial).
- Config SSOT (`IngestionPersistSettings`, `ChunkVectorBuildOptions::STANDARD`) prevents
  orchestrator/processor drift — verified by `contract_persist_config_parity_across_callers`.
- `make test-spec021` green — practical CI gate for this mission.

**Losses:**
- Persister covers **~25% of the original 8-step sequence** (chunk vectors + merge). KV
  chunks, relational stats, lineage, checkpoints remain in processor — second-class citizens
  that can still diverge in future edits.
- **Postgres UNWIND path** not E2E-tested through worker upload — memory contracts + unit
  tests only; production runs Postgres.
- P-G14 TOCTOU: concurrent uploads of same `pdf_id` can race before task row exists.
- Vision concurrency defaults (8→2 pages, semaphore) trade **4× latency** for OOM safety —
  correct engineering, must be in runbooks.

### 13.7 Plan-19 as a document — meta-critique

**Strengths:**
- Excellent RC → work-item traceability from file 18.
- Edge-case registry (§7) is usable for future PRs.
- §8 "what we are NOT doing" is honest about GraphRAG scope.

**Failures:**
- §2.2 trait model marketed SOLID completion that was **deferred by design** — readers who
  skim §2 without §13 will overestimate architecture maturity.
- §9 originally showed "After Phase H" metrics that were **aspirational**, not measured.
- §10–§12 embedded point-in-time assessments that contradicted §1 after P-G2 landed.
- P-G2 marked ✅ while acceptance criteria still referenced three callers and P-G12 tests —
  **status emoji lied** until this pass.

**Recommendation:** Treat plan-19 as a **closed correctness mission** with a **short tail**
(§6 remaining). Do not reopen the full 18-day schedule. Next ROI:
1. P-G4-graph batch in merger (perf + parity with vector batching)
2. P-G8-http contract (one test, high regression value)
3. P-G1b runbook for operators with legacy tenants

### 13.8 Verification snapshot (this re-assessment)

```bash
make test-spec021                                          # green
cargo test -p edgequake-pipeline --test contract_ingestion_persistence
cargo test -p edgequake-storage --test contract_workspace_scoped_analytics
cargo test -p edgequake-query --test contract_query_modes
cargo test -p edgequake-api --test e2e_spec021_ingestion_persister
```

### 13.9 Final grades (four lenses) — superseded by §13.10 and plan-25

Historical mid-closure grades; see **§13.10** and **`25-brutal-post-closure-assessment.md`** for current verdict.

### 13.10 Final re-assessment (plan-24 + continuation, rev.5)

| Lens | Grade | Verdict |
|------|-------|---------|
| GraphRAG | C+ | Corruption fixed; no community intelligence |
| LightRAG | A− | Batch merge + persister + deterministic worker E2E |
| AI Engineer | A | Worker + orchestrator (default) cache E2E; Mix HTTP weight engine-only |
| System Engineer | A− | RC-7 closed; persister scope partial by design |
| SOLID / DRY | A | Shared fixture; DIP ports; orchestrator default caches |

**Mission status:** Closed except P-G1b (admin). Full brutal detail: **`25-brutal-post-closure-assessment.md`** (rev.5).
