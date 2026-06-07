# SPEC-006: Resource Budget Catalog

**Spec ID:** `006-ensure-perf`  
**Status:** Draft  
**Purpose:** Target **single source of truth** for all resource caps.  
**Today:** Values are scattered (audit in [003](003_codebase_audit.md)).  
**Tomorrow:** `edgequake-core/src/resource_budget.rs` (proposed in [007](007_adr.md)).

---

## Catalog Rules

**BR-006-010:** Every cap in this table must have exactly one authoritative definition.
Handlers **must not** hardcode duplicate literals.

**BR-006-011:** Env vars override defaults via `from_env()` with documented min/max clamps.

**BR-006-012:** Changing a default requires updating this doc + regression tests in [008](008_regression_contract.md).

---

## 1. Memory & Materialization

| ID | Budget | Current value | Target SSOT | Current file | Env var |
|----|--------|---------------|-------------|--------------|---------|
| RB-MEM-001 | `GRAPH_SCAN_THRESHOLD_NODES` | *(missing)* | **50000** | *new* | `EDGEQUAKE_GRAPH_SCAN_THRESHOLD` |
| RB-MEM-002 | `MAX_GRAPH_MATERIALIZE_CONCURRENT` | *(missing)* | **1** | *new* | `EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT` |
| RB-MEM-003 | `GRAPH_NODE_RESPONSE_MAX` | 500 | 500 | `graph_types.rs:93` | — |
| RB-MEM-004 | `GRAPH_DEPTH_MAX` | 5 | 5 | `graph_types.rs:96` |
| RB-MEM-005 | `MAX_UPLOAD_BYTES` | **50 MB** (drift: 100 MB server) | **52428800** | `config.rs:72` | — |
| RB-MEM-006 | `MAX_QUERY_CHARS` | 10000 | 10000 | `config.rs:73` | — |
| RB-MEM-007 | `HEADROOM_RATIO` | *(missing)* | **0.75** | *new* | `EDGEQUAKE_MEM_HEADROOM_RATIO` |

---

## 2. Pipeline & Ingestion

| ID | Budget | Default | Min | Max | File | Env |
|----|--------|---------|-----|-----|------|-----|
| RB-ING-001 | `max_concurrent_extractions` | 16 | 1 | 256 | `pipeline/config.rs:69` | `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` |
| RB-ING-002 | `chunk_extraction_timeout_secs` | 180 | 10 | ∞ | `config.rs:54` | `EDGEQUAKE_CHUNK_TIMEOUT_SECS` |
| RB-ING-003 | `chunk_max_retries` | 3 | 0 | 20 | `config.rs:60` | `EDGEQUAKE_CHUNK_MAX_RETRIES` |
| RB-ING-004 | `chunk_token_size` | 1200 | — | — | `orchestrator/mod.rs:233` | — |
| RB-ING-005 | `chunk_overlap_tokens` | 100 | — | — | `mod.rs:234` | — |
| RB-ING-006 | `max_entities_per_chunk` | 20 | — | — | `core/config.rs:135` | — |
| RB-ING-007 | `max_relations_per_chunk` | 20 | — | — | `core/config.rs:136` | — |
| RB-ING-008 | `embedding_batch_size` | 100 | — | — | `pipeline/config.rs:150` | — |
| RB-ING-009 | `extraction_batch_size` | 10 | — | — | `pipeline/config.rs:149` | — |

---

## 3. Workers & Tasks

| ID | Budget | Default | File | Env |
|----|--------|---------|------|-----|
| RB-WRK-001 | `num_workers` | `max(4, cpus×4)` | `main.rs:575` | `WORKER_THREADS` |
| RB-WRK-002 | `max_tasks_per_tenant` | `max(1, workers×3/4)` | `main.rs:592` | `MAX_TASKS_PER_TENANT` |
| RB-WRK-003 | `processing_timeout_secs` | 7200 | `main.rs:602` | `TASK_PROCESSING_TIMEOUT_SECS` |
| RB-WRK-004 | `task_queue_capacity` | 100 | `postgres.rs:311` | *(proposed: `TASK_QUEUE_CAPACITY`)* |
| RB-WRK-005 | `MIN_PROCESSING_TIMEOUT_SECS` | 60 | `worker.rs:72` | — |

---

## 4. Database

| ID | Budget | Default | File | Env |
|----|--------|---------|------|-----|
| RB-DB-001 | `postgres.max_connections` | 32 | `postgres/config.rs:89` | `DATABASE_POOL_SIZE` *(proposed)* |
| RB-DB-002 | `graph_query_timeout_secs` | 15 | `traversal.rs:141` | `EDGEQUAKE_GRAPH_QUERY_TIMEOUT` *(proposed)* |
| RB-DB-003 | `statement_timeout` | PG server | migrations | `statement_timeout` |

**BR-006-013:** `RB-DB-001` ≥ `RB-WRK-001` + HTTP handler reserve (min 8).

---

## 5. LLM & Query Context

| ID | Budget | Default | File | Env |
|----|--------|---------|------|-----|
| RB-LLM-001 | `DEFAULT_MAX_TOKENS` | 16384 | `safety_limits.rs:27` | `EDGEQUAKE_LLM_MAX_TOKENS` |
| RB-LLM-002 | `ABSOLUTE_MAX_TOKENS` | 65536 | `safety_limits.rs:37` | — |
| RB-LLM-003 | `DEFAULT_TIMEOUT_SECS` | 600 | `safety_limits.rs:30` | `EDGEQUAKE_LLM_TIMEOUT_SECS` |
| RB-LLM-004 | `max_context_tokens` (SOTA) | 30000 | `sota_engine/mod.rs:152` | — |
| RB-LLM-005 | `max_entities` | 60 | `mod.rs:142` | — |
| RB-LLM-006 | `max_relationships` | 60 | `mod.rs:145` | — |
| RB-LLM-007 | `max_chunks` | 20 | `mod.rs:148` | — |
| RB-LLM-008 | `orchestrator max_token_*` | **30000** ✅ | `orchestrator/mod.rs`, `MAX_ORCHESTRATOR_CONTEXT_TOKENS` | — |

**Remediation RB-LLM-008:** ✅ Done P3 — uses `MAX_ORCHESTRATOR_CONTEXT_TOKENS` (same as RB-LLM-004).

---

## 6. PDF / Vision

| ID | Budget | Default | File | Env |
|----|--------|---------|------|-----|
| RB-PDF-001 | `vision_timeout_secs` | 600 | `docker-compose.yml:67` | `EDGEQUAKE_VISION_TIMEOUT_SECS` |
| RB-PDF-002 | `pdf_concurrency` | adaptive 1–8 | `pdf_processing.rs:91-106` | — |
| RB-PDF-003 | `pdf_dpi` | adaptive 96–150 | `pdf_processing.rs:108-116` | — |
| RB-PDF-004 | `huge_file_bytes` | 50 MB | `pdf_processing.rs:89` | — |
| RB-PDF-005 | `large_file_bytes` | 25 MB | `pdf_processing.rs:88` | — |

---

## 7. API Pagination

| ID | Budget | Default | File |
|----|--------|---------|------|
| RB-API-001 | `page_size_max` | 100 | `entity_crud.rs:54`, `relationships/list.rs:36` |
| RB-API-002 | `page_size_min` | 1 | same |
| RB-API-003 | `search_labels_limit` | 20 | `graph_types.rs:134` |

---

## 8. Caches

| ID | Budget | Default | File |
|----|--------|---------|------|
| RB-CACHE-001 | `keyword_cache_capacity` | 1000 | `sota_engine/mod.rs:341` |
| RB-CACHE-002 | `workspace_stats_ttl_secs` | 60 | `workspaces/stats.rs` |
| RB-CACHE-003 | `lineage_cache_max_entries` | see file | `lineage/cache.rs:18` |

---

## 9. Container (operational)

| ID | Budget | Recommended | File |
|----|--------|-------------|------|
| RB-OPS-001 | `mem_limit` | 4g (tune per deployment) | `docker-compose.yml` *(add)* |
| RB-OPS-002 | `cpus` | 4 | *(add)* |
| RB-OPS-003 | `OOM exit code` | 137 | Linux kernel |

---

## 10. DRY Migration Checklist

When implementing `ResourceBudgetConfig`:

- [ ] Replace `graph_types.rs` constants with `ResourceBudget::graph_*`
- [ ] Replace `entity_crud` page clamp with `ResourceBudget::clamp_page_size`
- [ ] Unify `MAX_UPLOAD_BYTES` — remove `server.rs:88` hardcoded 100 MB
- [ ] Export `from_env()` in one module; crates import, never duplicate `read_env_*`
- [ ] Log all resolved budgets at startup (structured JSON, one line)

---

## Cross-ref

| Consumer | Uses catalog section |
|----------|---------------------|
| [006 Architecture](006_architecture_remediation.md) | §1–3 for guard thresholds |
| [008 Regression](008_regression_contract.md) | Assert defaults match table |
| [009 Runbook](009_operator_runbook.md) | §9 ops tuning |
