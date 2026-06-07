# SPEC-006: Violation Registry — Edge Cases & Remediations

**Spec ID:** `006-ensure-perf`  
**Status:** Draft

Each entry: **Violation** → **Edge cases** → **Remediation** → **Regression test** → **Code refs**

---

## V-006-001 — Full-graph list with fake pagination

**Severity:** P0 CRITICAL  
**NFR:** NFR-006-001

### Violation

`list_entities` / `list_relationships` clamp `page_size` then load entire graph.

### Edge cases

| Case | Behavior today | Risk |
|------|----------------|------|
| page=1, page_size=10 on 500k nodes | Loads 500k, returns 10 | OOM |
| `entity_type` filter | Filter **after** full load | Wasted RAM |
| `search` query param | In-memory string match on all nodes | OOM + CPU |
| Concurrent list + delete | 2× full graph in RAM | OOM ×2 |
| Tenant with 0 nodes | Empty vec — OK | None |
| Memory backend (tests) | Same pattern — masks prod bug | False confidence |

### Remediation

**TR-006-001:** Add `GraphStorage::list_nodes_filtered(filter, offset, limit) -> (Vec, total_count)`.

**TR-006-002:** Postgres impl uses Cypher `WHERE` + `SKIP`/`LIMIT` + `COUNT` subquery.

**TR-006-003:** Handler calls pushed-down API only; remove `get_all_nodes` from `entity_crud.rs`.

### Regression

`resource_safety_list_entities_bounded_memory` — [008](008_regression_contract.md)

### Code refs

`entity_crud.rs:58-62`, `relationships/list.rs:40-43`, `graph_read_ops.rs:28`

---

## V-006-002 — Document delete triple materialization

**Severity:** P0 CRITICAL  
**NFR:** NFR-006-002

### Violation

Single delete loads all nodes, all edges, then all nodes again for orphan check.

### Edge cases

| Case | Behavior | Risk |
|------|----------|------|
| Shared entity (multi-doc sources) | Must update, not delete | Correctness — keep |
| Orphan edge after node delete | Needs edge scan | Can't load all nodes twice |
| Bulk delete N docs | N × triple scan if sequential | N × OOM |
| Delete during active ingestion | Race on graph | Locking / transaction |
| Empty workspace delete | 3 empty vecs | OK |
| 200k nodes, 1 doc with 5 entities | Still scans 200k | **Primary OOM path** |

### Remediation

**TR-006-004:** `find_nodes_by_source_id(doc_id) -> Vec<NodeId>` indexed query.

**TR-006-005:** `find_edges_by_source_id(doc_id)` + `find_orphan_edges(node_ids)` batch.

**TR-006-006:** Single pass; no `get_all_nodes` in delete path.

### Regression

`resource_safety_delete_document_large_graph` — mock 100k nodes, assert RSS delta < threshold

### Code refs

`documents/delete/single.rs:272-338`, `orchestrator/deletion.rs:54-216`

---

## V-006-003 — Graph query timeout fallback loads full graph

**Severity:** P0 CRITICAL  
**NFR:** NFR-006-003

### Violation

On 15s timeout, fallback calls `get_all_nodes()` then `.take(500)`.

### Edge cases

| Case | Behavior | Risk |
|------|----------|------|
| Large graph + slow AGE | Timeout → OOM | **Exact 137 scenario** |
| Small graph + timeout | Overkill but survives | Masks bug |
| `start_node` traversal timeout | Different code path | Audit separately |
| Client requests max_nodes=500 | Pays for millions loaded | AX-03 |

### Remediation

**TR-006-007:** On timeout → `503` + `Retry-After: 30` + partial empty graph metadata.

**TR-006-008:** Optional degraded mode: `get_nodes_batch` on precomputed popular IDs cache.

**BR-006-014:** Fallback must never call `get_all_*`.

### Regression

`resource_safety_graph_timeout_no_full_load` — inject slow mock, assert no `get_all_nodes` call

### Code refs

`graph_query/traversal.rs:156-198`

---

## V-006-004 — Lineage scans full graph per request

**Severity:** P0 HIGH  
**NFR:** NFR-006-004

### Edge cases

| Case | Risk |
|------|------|
| Open lineage drawer on document detail page | Background full scan |
| `chunk_detail` + `queries` same session | Duplicate scans |
| Document with 0 entities | Full scan for nothing |

### Remediation

**TR-006-009:** Lineage endpoints use `source_id` index queries (same as delete).

### Code refs

`lineage/queries.rs:152-185`, `lineage/chunk_detail.rs:112-141`

---

## V-006-005 — Community detection O(n²) on full graph

**Severity:** P1 HIGH

### Edge cases

| Case | Risk |
|------|------|
| Global query triggers community rebuild | Background CPU + RAM |
| `max_iterations` default on 100k nodes | Minutes + GB adjacency |

### Remediation

**TR-006-010:** Gate: `node_count_fast() > GRAPH_SCAN_THRESHOLD` → skip with warning.

**TR-006-011:** Run async job with dedicated worker + memory budget.

### Code refs

`storage/community.rs:145-220`

---

## V-006-006 — Worker × extraction multiplication

**Severity:** P1 HIGH  
**NFR:** NFR-006-005

### Edge cases

| Case | Risk |
|------|------|
| 32 workers each processing 50MB doc | 1.6 GB doc text in flight |
| Burst upload 50 PDFs | Queue 100, workers 32, RAM spike |
| Single tenant monopolizes | Mitigated by `TenantConcurrencyLimiter` |
| `MAX_TASKS_PER_TENANT=0` | Disables limiter — document env semantics |

### Remediation

**TR-006-012:** Startup log: `workers × max_concurrent × avg_doc_size` estimate.

**TR-006-013:** Production defaults: `WORKER_THREADS=8`, `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=4`.

### Code refs

`main.rs:572-592`, `pipeline/extraction.rs:41-43`, `tenant_limiter.rs:46-66`

---

## V-006-007 — Postgres pool starvation

**Severity:** P1 MEDIUM

### Edge cases

| Case | Risk |
|------|------|
| 32 workers × 16 extractions need DB | Pool 32 — deadlock wait |
| Long-running graph query holds connection | Starves workers |
| `sqlx` acquire timeout | Cascading retries |

### Remediation

**TR-006-014:** Separate read pool for analytics vs write pool for ingestion.

**BR-006-013:** Pool size formula in [004](004_resource_budget_catalog.md) RB-DB-001.

### Code refs

`postgres/config.rs:82-89`, `connection.rs:69`

---

## V-006-008 — PDF page buffer spike

**Severity:** P1 MEDIUM

### Edge cases

| Case | Risk |
|------|------|
| 32 parallel PDF tasks × 8 pages × 5MB | ~1.2 GB image buffers |
| 1000-page doc at 150 DPI | Mitigated to concurrency=1, dpi=96 |
| Vision timeout 600s | Long-held buffers |

### Remediation

Already partial — enforce `compute_safe_pdf_resource_profile` globally.

**TR-006-015:** Cross-check worker count vs PDF concurrency in startup validation.

### Code refs

`pdf_processing.rs:80-118`

---

## V-006-009 — Unbounded audit / task channels

**Severity:** P2 LOW

### Edge cases

| Case | Risk |
|------|------|
| DB down + audit flood | Unbounded mpsc growth |
| Task queue 100 full + producers | Backpressure via `send().await` |

### Remediation

**TR-006-016:** Audit channel → bounded with drop-oldest policy + metric `audit_dropped_total`.

### Code refs

`audit/logger.rs:36`, `tasks/queue.rs:127`

---

## V-006-010 — Orchestrator 100k token budgets

**Severity:** P2 MEDIUM

### Edge cases

| Case | Risk |
|------|------|
| Orchestrator path vs SOTA path | Different caps — confusion |
| 3 × 100k token contexts assembled | ~1.2 MB text per query |

### Remediation

**TR-006-017:** Single `max_context_tokens` from `ResourceBudget`; orchestrator reads SOTA default.

### Code refs

`orchestrator/mod.rs:230-232`, `sota_engine/mod.rs:152`

---

## V-006-011 — Rate limiter DashMap growth

**Severity:** P3 LOW

### Edge cases

| Case | Risk |
|------|------|
| Unique tenant key per request | Map grows unbounded |
| `tenant_limiter` semaphore map | Periodic cleanup exists |

### Remediation

**TR-006-018:** TTL eviction on rate limiter buckets (already noted in `tenant_limiter.rs:122`).

### Code refs

`rate-limiter/limiter.rs:104`, `tenant_limiter.rs:122`

---

## V-006-012 — Upload body limit drift (50 vs 100 MB)

**Severity:** P1 MEDIUM  
**NFR:** NFR-006-006

### Violation

`AppConfig` 50 MB; Axum layer 100 MB.

### Edge cases

| Case | Risk |
|------|------|
| 75 MB upload | Accepted by server, may fail later in processor |
| Client SDK uses 50 MB constant | Inconsistent error messages |

### Remediation

**TR-006-019:** `DefaultBodyLimit::max(ResourceBudget::max_upload_bytes())`.

### Code refs

`server.rs:88`, `state/config.rs:72`

---

## V-006-013 — No Docker memory limit

**Severity:** P1 OPERATIONAL  
**OR:** OR-006-001

### Remediation

Add `mem_limit: 4g` + document tuning in [009](009_operator_runbook.md).

### Code refs

`docker/docker-compose.yml` (edgequake service)

---

## Violation Priority Matrix

```
Impact →
         Low      Medium     High       Critical
      ┌────────┬──────────┬──────────┬───────────┐
High  │ V-011  │ V-006    │ V-005    │ V-001     │
Prob  │ V-009  │ V-008    │ V-004    │ V-002     │
      ├────────┼──────────┼──────────┼───────────┤
Med   │        │ V-010    │ V-007    │ V-003     │
      ├────────┼──────────┼──────────┼───────────┤
Low   │        │ V-012    │ V-013    │           │
      └────────┴──────────┴──────────┴───────────┘
```
