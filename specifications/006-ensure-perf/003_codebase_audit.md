# SPEC-006: Codebase Audit — Code Is Law

**Spec ID:** `006-ensure-perf`  
**Status:** Draft  
**Audit date:** 2026-06-06  
**Rule:** Every claim cites a file. Comments are evidence; **runtime paths are law**.

---

## 1. Existing Defenses (verified working)

### 1.1 Graph API caps

| Constant | Value | File |
|----------|-------|------|
| `MAX_GRAPH_NODES` | 500 | `edgequake-api/src/handlers/graph_types.rs:93` |
| `MAX_GRAPH_DEPTH` | 5 | `edgequake-api/src/handlers/graph_types.rs:96` |
| `GraphQueryParams::validated()` | clamp | `graph_types.rs:111-114` |

**Scope:** Graph viewer + stream endpoints only. Does **not** protect list/delete/lineage.

### 1.2 List API pagination (illusory)

| Handler | page_size clamp | Still loads all? | File |
|---------|-----------------|------------------|------|
| `list_entities` | `[1, 100]` | **YES** `get_all_nodes()` | `entity_crud.rs:53-62` |
| `list_relationships` | `[1, 100]` | **YES** `get_all_edges()` | `relationships/list.rs:35-43` |

### 1.3 Pipeline concurrency

| Setting | Default | Env override | File |
|---------|---------|--------------|------|
| `max_concurrent_extractions` | 16 | `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` (1–256) | `pipeline/config.rs:69,128-133` |
| `chunk_extraction_timeout_secs` | 180 | `EDGEQUAKE_CHUNK_TIMEOUT_SECS` | `pipeline/config.rs:54,110-115` |
| `DEFAULT_MAX_TOKENS` | 16384 | `EDGEQUAKE_LLM_MAX_TOKENS` (cap 65536) | `api/safety_limits.rs:27-37` |

### 1.4 Worker pool

| Setting | Default | Env | File |
|---------|---------|-----|------|
| `num_workers` | `max(4, num_cpus×4)` | `WORKER_THREADS` | `main.rs:572-575` |
| `max_tasks_per_tenant` | `max(1, num_workers×3/4)` | `MAX_TASKS_PER_TENANT` | `main.rs:589-592` |
| `processing_timeout_secs` | 7200 | `TASK_PROCESSING_TIMEOUT_SECS` (min 60) | `main.rs:598-612` |
| Task queue capacity | 100 | — | `state/postgres.rs:311` |

### 1.5 PDF adaptive throttling

`compute_safe_pdf_resource_profile()` — `pdf_processing.rs:80-118`

| Condition | concurrency | dpi |
|-----------|-------------|-----|
| local + huge/200+ pages | 1–2 | 96–150 |
| cloud + 1000+ pages | 1 | 96 |

### 1.6 Query context budgets (SOTA engine)

| Field | Default | File |
|-------|---------|------|
| `max_entities` | 60 | `query/sota_engine/mod.rs:142` |
| `max_relationships` | 60 | `mod.rs:145` |
| `max_chunks` | 20 | `mod.rs:148` |
| `max_context_tokens` | 30000 | `mod.rs:152` |

### 1.7 Fast counts (safe metadata)

| Method | Complexity | File |
|--------|------------|------|
| `node_count_fast()` | O(1) estimate | `storage/.../analytics_ops.rs:52` |
| `edge_count_fast()` | O(1) estimate | `analytics_ops.rs:56` |
| `pg_get_workspace_stats` | SQL subquery COUNT | `workspace_ops.rs:441-455` |

### 1.8 Bounded caches

| Cache | Bound | File |
|-------|-------|------|
| Keyword cache | 1000 entries | `sota_engine/mod.rs:341` |
| Conversation TTL LRU | capacity + TTL | `cache_manager.rs:55-64` |
| Lineage cache | max entries + eviction | `lineage/cache.rs:18,58` |
| Workspace stats cache | 60s TTL | `workspaces/stats.rs:44-51` |

### 1.9 Upload / body limits (DRIFT)

| Layer | Limit | File |
|-------|-------|------|
| `AppConfig::max_document_size` | 50 MB | `state/config.rs:72` |
| `ApiConfig::body_limit` | 50 MB | `core/config.rs:209` |
| Axum `DefaultBodyLimit` | **100 MB** | `server.rs:88` |

**Violation V-006-012** — see [005](005_violation_registry.md).

### 1.10 Orchestrator token budgets (HIGH)

| Field | Default | File |
|-------|---------|------|
| `max_token_for_text_unit` | **100000** | `core/orchestrator/mod.rs:230` |
| `max_token_for_global_context` | **100000** | `mod.rs:231` |
| `max_token_for_local_context` | **100000** | `mod.rs:232` |

Conflicts with SOTA `max_context_tokens: 30000`.

---

## 2. Unbounded Loaders — `get_all_nodes` / `get_all_edges`

### 2.1 Storage implementation (root)

```rust
// nodes_ops.rs:287-288
let cypher = "MATCH (n:Node) RETURN n";

// edges_ops.rs:175-176
let cypher = "MATCH ()-[r:EDGE]->() RETURN r";
```

Exposed via trait `GraphStorageReadOps` — `graph_read_ops.rs:28,66`.

### 2.2 Production call sites (API layer) — MUST remediate

| File | Calls | Trigger |
|------|-------|---------|
| `entity_crud.rs:62` | `get_all_nodes` | GET /entities |
| `relationships/list.rs:43` | `get_all_edges` | GET /relationships |
| `relationships/get.rs:41` | `get_all_nodes` | GET /relationships/:id |
| `relationships/delete.rs` | `get_all_nodes` | DELETE |
| `relationships/update.rs` | `get_all_nodes` | PATCH |
| `documents/delete/single.rs:272,335,338` | nodes×2 + edges | DELETE document |
| `documents/delete/bulk.rs:204,235,236` | nodes + edges×2 | Bulk delete |
| `documents/delete/impact.rs:70,88` | nodes + edges | Impact preview |
| `lineage/queries.rs:152,185,343,354` | nodes + edges | Lineage API |
| `lineage/chunk_detail.rs:112,141` | nodes + edges | Chunk lineage |
| `lineage/entity_provenance.rs` | `get_all_nodes` | Provenance |
| `graph_query/traversal.rs:170,190` | `get_all_nodes` | **Timeout fallback** |
| `pdf_upload/helpers.rs:216` | `get_all_nodes` | PDF helper |
| `documents/storage_helpers.rs` | `get_all_*` | Storage helpers |

### 2.3 Core / storage call sites — gate or async

| File | Calls | Notes |
|------|-------|-------|
| `orchestrator/deletion.rs` | nodes + edges | Shared with API delete |
| `community.rs:149-150,314-315,412-413` | nodes + edges | Louvain O(n²) risk |
| `graph_analytics_ops.rs` | via defaults | Trait default impls |

### 2.4 Allowlisted (tests/benches/examples)

| Path | Reason |
|------|--------|
| `benches/storage_bench.rs` | Benchmark only |
| `examples/graph_exploration.rs` | Dev tool |
| `tests/e2e_*`, `tests/graph_*` | CI with small graphs |
| `adapters/memory/graph.rs` | In-memory test backend |

---

## 3. Dangerous Fallback Pattern

`graph_query/traversal.rs:156-198`:

1. Primary: `get_popular_nodes_with_degree(max_nodes)` with 15s timeout
2. On DB timeout → **`get_all_nodes()`** then `.take(max_nodes)`

**AX-02 violation:** Fallback is strictly worse than failure.

---

## 4. Concurrency Stack (multiplication proof)

Default 8-core machine:

```text
workers           = 32
extractions/doc   = 16
pool connections  = 32
max in-flight LLM = up to 32 × 16 = 512 (theoretical; IO-bound)

RAM per extraction ≈ chunk (1200 tokens) + JSON output (≤16k tokens)
                   ≈ 50–200 KB minimum × concurrent
```

---

## 5. Docker / Ops gap

`docker/docker-compose.yml` — **no** `mem_limit`, `deploy.resources.limits`, or
`OOMScoreAdjust` on `edgequake` service (lines 9–90).

---

## 6. Audit Commands (reproducible)

```bash
# Unbounded loaders in API
rg 'get_all_nodes\(\)|get_all_edges\(\)' edgequake/crates/edgequake-api/src

# Body limit drift
rg 'max_document_size|DefaultBodyLimit|body_limit' edgequake/

# Concurrency defaults
rg 'DEFAULT_MAX_CONCURRENT|WORKER_THREADS|max_connections' edgequake/

# Graph caps
rg 'MAX_GRAPH_NODES|MAX_GRAPH_DEPTH' edgequake/
```

---

## 7. Cross-refs

| Finding | Violation ID | Remediation |
|---------|--------------|-------------|
| Full graph list | V-006-001 | [006](006_architecture_remediation.md) §3.1 |
| Delete triple load | V-006-002 | [006](006_architecture_remediation.md) §3.2 |
| Graph fallback | V-006-003 | [006](006_architecture_remediation.md) §3.3 |
| Body limit drift | V-006-012 | [004](004_resource_budget_catalog.md) |
| Orchestrator 100k tokens | V-006-010 | [004](004_resource_budget_catalog.md) |
