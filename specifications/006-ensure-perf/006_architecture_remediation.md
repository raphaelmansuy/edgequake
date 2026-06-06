# SPEC-006: Architecture Remediation — DRY & SOLID

**Spec ID:** `006-ensure-perf`  
**Status:** Draft  
**Decision record:** [007_adr.md](007_adr.md)

---

## 1. Design Goals

| Goal | Mechanism |
|------|-----------|
| Never OOM on request path | Push-down queries + pre-flight gates |
| DRY budgets | `ResourceBudgetConfig` SSOT ([004](004_resource_budget_catalog.md)) |
| SOLID boundaries | `ResourceGuard` (S), `GraphScanOps` (I), trait deps (D) |
| Zero regression | Contracts in [008](008_regression_contract.md) |
| Incremental migration | Deprecate `get_all_*`; allowlist shrinks per phase |

---

## 2. Module Layout (proposed)

```text
edgequake-core/src/resource/
├── budget.rs          # ResourceBudgetConfig + from_env()  [S: one module]
├── guard.rs           # ResourceGuard trait + AdmissionDecision
├── estimate.rs        # OperationCost profiles (OCP registry)
└── mod.rs

edgequake-storage/src/traits/
├── graph_scan_ops.rs  # NEW: list_nodes_filtered, find_by_source_id
└── graph_read_ops.rs  # get_all_* → #[deprecated] + debug_assert in debug builds

edgequake-api/src/
├── resource_guard.rs  # Axum layer + AppState injection
└── handlers/          # migrate one-by-one
```

**DIP:** Handlers receive `Arc<dyn ResourceGuard>` and `GraphReadView` — never call
`get_all_nodes` directly after migration.

---

## 3. Remediation by Violation

### 3.1 V-006-001 — List entities/relationships

#### New trait (`GraphScanOps`)

```rust
// SPEC-006: TR-006-001
#[async_trait]
pub trait GraphScanOps: Send + Sync {
    async fn list_nodes_filtered(
        &self,
        filter: NodeFilter,      // tenant, workspace, entity_type, search
        offset: usize,
        limit: usize,
    ) -> Result<PagedResult<GraphNode>>;

    async fn list_edges_filtered(
        &self,
        filter: EdgeFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedResult<GraphEdge>>;
}
```

#### Postgres implementation sketch

```cypher
MATCH (n:Node)
WHERE n.tenant_id = $tenant AND n.workspace_id = $ws
  AND ($type IS NULL OR n.entity_type = $type)
  AND ($q IS NULL OR n.node_id CONTAINS $q)
RETURN n
SKIP $offset LIMIT $limit
```

Separate `COUNT` query for `total` — still O(1) index scan vs O(n) materialize.

#### Handler migration (SRP)

`entity_crud.rs` — **only** parse query → call `list_nodes_filtered` → map response.
Tenant filter moves to SQL (today: `filter_nodes_by_tenant_context` post-load).

**DRY:** `clamp_page_size()` from `ResourceBudget`; remove duplicate `.clamp(1, 100)`.

---

### 3.2 V-006-002 — Document delete

#### New methods

```rust
// SPEC-006: TR-006-004
async fn find_nodes_by_source_prefix(
    &self,
    tenant_ctx: &TenantContext,
    prefixes: &[String],   // doc_id, chunk keys
) -> Result<Vec<GraphNode>>;

async fn find_edges_by_source_prefix(...) -> Result<Vec<GraphEdge>>;

async fn find_orphan_edges_for_nodes(
    &self,
    node_ids: &[String],
) -> Result<Vec<GraphEdge>>;
```

#### Delete algorithm (single pass)

```text
1. find_nodes_by_source_prefix(doc)     → affected_nodes
2. For each node: update sources or delete
3. find_edges_by_source_prefix(doc)     → affected_edges
4. find_orphan_edges_for_nodes(deleted) → orphan_edges (indexed, not full scan)
5. Batch delete vectors via workspace storage
```

**Removes:** `get_all_nodes` ×2, `get_all_edges` ×1 from `single.rs`.

**Shared logic:** Extract `DocumentGraphCascade` service used by API delete +
`orchestrator/deletion.rs` (DRY D1).

---

### 3.3 V-006-003 — Graph timeout fallback

Replace fallback block in `traversal.rs:156-198`:

```rust
// SPEC-006: BR-006-014
Err(GraphQueryError::Timeout) => {
    return Err(ApiError::ServiceUnavailable {
        retry_after_secs: 30,
        message: "Graph query exceeded time budget".into(),
    });
}
```

**Optional degraded path:** Return cached `popular_node_ids` from Redis/KV (bounded 500),
fetch via `get_nodes_batch` — never full scan.

---

### 3.4 V-006-004 — Lineage

Refactor `lineage/queries.rs` to use `find_nodes_by_source_prefix` /
`find_edges_by_source_prefix`. Same service as §3.2 (DRY).

---

### 3.5 Global admission control

```rust
// SPEC-006: NFR-006-003
pub struct GraphMaterializationGuard {
    semaphore: Semaphore,  // RB-MEM-002, default 1
    threshold: usize,      // RB-MEM-001, default 50000
}

impl GraphMaterializationGuard {
    pub async fn admit(&self, graph: &GraphReadView, op: GraphOp) -> Result<AdmissionTicket> {
        let count = graph.node_count_fast().await?;
        if count > self.threshold && op.requires_full_scan() {
            return Err(ResourceError::GraphTooLarge { count, threshold: self.threshold });
        }
        let _permit = self.semaphore.acquire().await?;
        Ok(AdmissionTicket { _permit })
    }
}
```

**OCP:** `OperationCost` enum registers new ops without changing guard:

```rust
enum GraphOp {
    ListEntities,
    DeleteDocument,
    LineageQuery,
    CommunityDetection,  // requires_full_scan = true
    // ...
}
```

---

## 4. SOLID Mapping

| Principle | Implementation |
|-----------|----------------|
| **S** | `ResourceGuard` — admission only; `DocumentGraphCascade` — delete only |
| **O** | Register `OperationCost` profiles; add handlers without editing guard |
| **L** | Memory adapter implements `GraphScanOps` with same paging semantics |
| **I** | Split `GraphScanOps` from 40-method `GraphStorage`; handlers take scan trait |
| **D** | `list_entities` depends on `Arc<dyn GraphScanOps>`, not Postgres concrete |

---

## 5. Deprecation Policy for `get_all_*`

| Phase | Policy |
|-------|--------|
| P0 | `#[deprecated(note = "SPEC-006: use GraphScanOps")]` on trait methods |
| P0 | `debug_assert!` in debug builds when called from `edgequake-api` |
| P1 | CI grep gate — fail if new calls in `edgequake-api/src` |
| P2 | Remove from public trait; keep `pub(crate)` for tests/benches allowlist |

---

## 6. Implementation Order (no regression)

```text
Week 1: ResourceBudgetConfig + startup logging + body limit fix (V-006-012)
Week 2: GraphScanOps trait + Postgres list_nodes_filtered + entity_crud migration
Week 3: find_by_source_prefix + delete refactor + orchestrator DRY
Week 4: Remove graph fallback + lineage migration + GraphMaterializationGuard
Week 5: CI resource-proof suite + Docker mem_limit + docs
```

Each week: existing `cargo test` + `e2e_document_deletion` + new resource test green.

---

## 7. Files Touched (estimate)

| Crate | Files | LOC Δ |
|-------|-------|-------|
| `edgequake-core` | +4 new resource module | +400 |
| `edgequake-storage` | +graph_scan_ops, nodes/edges filtered SQL | +600 |
| `edgequake-api` | ~15 handlers, +resource_guard | -800 net (remove scans) |
| `edgequake-core` | orchestrator/deletion.rs refactor | -200 net |
| `docker` | compose mem_limit | +5 |

---

## Cross-refs

- Budget values: [004](004_resource_budget_catalog.md)
- ADR decision: [007](007_adr.md)
- Tests: [008](008_regression_contract.md)
- Violations: [005](005_violation_registry.md)
