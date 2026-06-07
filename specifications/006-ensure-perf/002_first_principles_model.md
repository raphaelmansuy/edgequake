# SPEC-006: First-Principles Resource Model

**Spec ID:** `006-ensure-perf`  
**Status:** Draft

---

## 1. Core Invariant

Every request and background job must satisfy **before execution**:

```text
estimated_peak_bytes × concurrency_factor  ≤  available_headroom_bytes
```

Where:

| Symbol | Definition | Code anchor |
|--------|------------|-------------|
| `estimated_peak_bytes` | Max transient allocation for this operation | Per-operation table (§3) |
| `concurrency_factor` | In-flight copies of same operation class | Workers, semaphores, HTTP pool |
| `available_headroom_bytes` | `mem_limit × HEADROOM_RATIO` (default 0.75) | [009](009_operator_runbook.md) |

**BR-006-001:** If estimate exceeds headroom → **fail fast** with `503 Service Unavailable`
and `Retry-After`, never proceed and hope.

---

## 2. Decomposition Law

Resource use is multiplicative, not additive:

```text
PeakRAM ≈ Σ (objects_loaded × bytes_per_object × copy_count)
```

### objects_loaded

| Pattern | Bounded? | Code |
|---------|----------|------|
| `MATCH (n:Node) RETURN n` | **NO** | `nodes_ops.rs:287` |
| `get_popular_nodes_with_degree(limit)` | YES | `traversal.rs:143` |
| `node_count_fast()` | YES (scalar) | `analytics_ops.rs:52` |
| Vector `query_filtered(..., top_k)` | YES | `vector_queries.rs:32` |

### copy_count

Rust `Vec` clones, JSON serialization, and overlapping requests multiply copies.
**BR-006-002:** An operation that loads graph data must not hold more than **one**
full materialization per request scope.

### concurrency_factor

```text
effective_parallelism = min(
    WORKER_THREADS,
    MAX_TASKS_PER_TENANT × active_tenants,
    max_concurrent_extractions × active_documents,
    postgres_pool.max_connections
)
```

**Code anchors:**

- Workers: `main.rs:572` — default `num_cpus * 4`
- Extractions: `pipeline/config.rs:69` — default `16`
- Pool: `postgres/config.rs:89` — default `32`
- Task queue: `postgres.rs:311` — `ChannelTaskQueue::new(100)`

**BR-006-003:** `effective_parallelism` must be computed at startup and logged; mismatch
(pool < workers × extractions) must emit **WARN** (already implicit; make explicit in NFR).

---

## 3. Operation Cost Table (estimates)

Use for guard decisions. Calibrate with benchmarks in [008](008_regression_contract.md).

| Operation class | objects_loaded | bytes/object (p50) | copies | Risk |
|-----------------|----------------|-------------------|--------|------|
| `list_entities` page 1 | **all nodes** | 2 KB | 1–2 | **CRITICAL** |
| `delete_document` | **all nodes + edges ×2** | 2 KB / 1.5 KB | 3 | **CRITICAL** |
| `lineage/queries` | **all nodes + edges** | 2 KB / 1.5 KB | 2 | **CRITICAL** |
| Graph traversal (happy) | `max_nodes` ≤ 500 | 2 KB | 1 | LOW |
| Graph traversal (fallback) | **all nodes** | 2 KB | 1 | **CRITICAL** |
| Community Louvain | **all nodes + edges** | 2 KB + adjacency | 2+ | HIGH |
| PDF vision (1 doc) | `page_count × dpi²` | ~0.5–5 MB/page | `concurrency` | MEDIUM |
| Query hybrid | 60 entities + 60 rels + 20 chunks | tokens | 1 | LOW |
| Workspace stats | SQL COUNT | O(1) | 1 | LOW |

### Worked example (production 137 scenario)

```text
nodes = 200_000, edges = 800_000
delete_document peak ≈ (200k×2KB) + (800k×1.5KB) + (200k×2KB)
                    ≈ 400 MB + 1.2 GB + 400 MB ≈ 2.0 GB

2 concurrent deletes or delete + list_entities ≈ 4+ GB
Default Docker (no mem_limit) → host OOM → exit 137
```

---

## 4. Defense Layers (ordered)

```text
┌─────────────────────────────────────────────────────────────┐
│ L1  Budget SSOT (ResourceBudgetConfig)     — prevent drift  │
├─────────────────────────────────────────────────────────────┤
│ L2  Push-down queries (SQL LIMIT/WHERE)    — don't load all │
├─────────────────────────────────────────────────────────────┤
│ L3  Pre-flight count (node_count_fast)     — reject early    │
├─────────────────────────────────────────────────────────────┤
│ L4  Global semaphores (graph materialize)  — cap concurrent │
├─────────────────────────────────────────────────────────────┤
│ L5  cgroup mem_limit + alerts              — last resort     │
└─────────────────────────────────────────────────────────────┘
```

**Principle:** L1–L4 prevent OOM; L5 contains blast radius. Never rely on L5 alone.

---

## 5. DRY Law for Resource Code

| Anti-pattern | DRY violation | Remediation |
|--------------|---------------|-------------|
| `page_size.clamp(1, 100)` in each handler | D3 config duplication | `ResourceBudget::clamp_page_size()` |
| `get_all_nodes` + filter in 12 handlers | D1 logic duplication | `GraphStorage::list_nodes_filtered()` |
| 50 MB vs 100 MB body limit | D3 config drift | Single `MAX_UPLOAD_BYTES` constant |
| Per-handler timeout constants | D3 | `ResourceBudget::graph_query_timeout()` |

---

## 6. SOLID Law for Resource Code

| Principle | Requirement |
|-----------|-------------|
| **S** | `ResourceGuard` — one reason to change: admission control |
| **O** | New operation types register cost profile without editing guard internals |
| **L** | Memory and Postgres `GraphStorage` both honor `list_nodes_filtered` contract |
| **I** | Split `GraphStorageReadOps` — fat `get_all_*` deprecated on new `GraphScanOps` |
| **D** | Handlers depend on `ResourceGuard` trait, not raw `get_all_nodes` |

See [006](006_architecture_remediation.md) and [007](007_adr.md).

---

## 7. Edge-Case Axioms

| Axiom | Statement |
|-------|-----------|
| **AX-01** | Pagination without push-down is **not** pagination |
| **AX-02** | Fallback paths must never be **more expensive** than primary |
| **AX-03** | `take(N)` after `get_all` still paid for `get_all` |
| **AX-04** | Bounded channel does not bound **memory per task** |
| **AX-05** | Fast count ≠ safe to load (200k nodes still fatal) |
| **AX-06** | Retries multiply resource use (backoff + re-execution) |

Full edge-case catalog: [005](005_violation_registry.md).
