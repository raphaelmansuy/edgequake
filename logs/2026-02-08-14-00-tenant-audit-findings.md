# Multi-Tenant Security Audit - CRITICAL FINDINGS

**Date**: 2026-02-08 14:00  
**Status**: 🚨 **CRITICAL VULNERABILITIES FOUND**  
**Auditor**: Automated audit of ingestion, query, and aggregation pipelines

## Executive Summary

Found **5 critical multi-tenant data leakage vulnerabilities** across cost tracking, graph visualization, and document listing endpoints. These vulnerabilities allow one tenant to access another tenant's sensitive data including costs, documents, and knowledge graph entities.

---

## CRITICAL Vulnerabilities (P0 - Fix Immediately)

### 1. Cost Summary Leakage (P0 - CRITICAL)

**File**: `edgequake/crates/edgequake-api/src/handlers/costs.rs`  
**Function**: `get_cost_summary()` (line 107-210)  
**Endpoint**: `GET /api/v1/costs/summary`

**Issue**: NO tenant context parameter - returns costs for ALL tenants.

**Code**:

```rust
pub async fn get_cost_summary(
    State(state): State<AppState>,
    // ❌ MISSING: tenant_ctx: TenantContext
) -> ApiResult<Json<WorkspaceCostSummaryResponse>> {
    // Queries ALL metadata keys without filtering
    let keys = state.kv_storage.keys().await?;
    let metadata_keys: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();
    // ... aggregates costs from all tenants
}
```

**Impact**:

- ✅ **TenantA request**: Shows costs for ALL tenants combined
- ✅ **TenantB request**: Shows same costs (ALL tenants)
- ❌ **Result**: Financial data leakage across tenants

**Sensitive Data Exposed**:

- Total costs across all tenants
- Document counts
- Token usage
- Operation breakdowns (extraction, embedding)
- Average cost per document

---

### 2. Cost History Leakage (P0 - CRITICAL)

**File**: `edgequake/crates/edgequake-api/src/handlers/costs.rs`  
**Function**: `get_cost_history()` (line 269-390)  
**Endpoint**: `GET /api/v1/costs/history`

**Issue**: NO tenant context parameter - returns historical costs for ALL tenants.

**Code**:

```rust
pub async fn get_cost_history(
    State(state): State<AppState>,
    Query(params): Query<CostHistoryQuery>,
    // ❌ MISSING: tenant_ctx: TenantContext
) -> ApiResult<Json<Vec<CostHistoryPoint>>> {
    // Queries ALL metadata without tenant filtering
    let keys = state.kv_storage.keys().await?;
    // ... aggregates cost trends for all tenants
}
```

**Impact**:

- Historical cost trends exposed across tenants
- Time-series data leakage (hourly, daily, weekly, monthly)
- Pattern analysis of other tenants' usage

**Sensitive Data Exposed**:

- Cost trends over time
- Usage patterns
- Document processing timestamps
- Token consumption history

---

### 3. Budget Status Endpoint (P1 - MEDIUM)

**File**: `edgequake/crates/edgequake-api/src/handlers/costs.rs`  
**Functions**: `get_budget_status()`, `update_budget()` (line 228-254)  
**Endpoints**:

- `GET /api/v1/costs/budget`
- `PATCH /api/v1/costs/budget`

**Issue**: Returns dummy data currently, but NO tenant isolation when implemented.

**Code**:

```rust
pub async fn get_budget_status(
    State(_state): State<AppState>
    // ❌ MISSING: tenant_ctx: TenantContext
) -> ApiResult<Json<BudgetInfo>> {
    // TODO: Currently returns dummy data
    Ok(Json(BudgetInfo {
        monthly_budget_usd: 100.0,
        // ...
    }))
}
```

**Impact**:

- Currently low (dummy data)
- **HIGH** when implemented - budget settings leakage

---

### 4. Graph Visualization Permissive Filtering (P0 - CRITICAL)

**File**: `edgequake/crates/edgequake-api/src/handlers/graph.rs`  
**Function**: `get_graph()` (line 1095-1124)  
**Endpoint**: `GET /api/v1/graph`

**Issue**: Uses OLD permissive tenant filtering logic (allows None).

**Code**:

```rust
let matches_tenant_context =
    |properties: &std::collections::HashMap<String, serde_json::Value>| {
        // ❌ PERMISSIVE: If no tenant context, allow all nodes
        if tenant_ctx.tenant_id.is_none() {
            return true;
        }
        // ... more permissive checks
        // ❌ BACKWARD COMPATIBILITY: "If node has no tenant_id but context has one, still include it"
    };
```

**Impact**:

- Admin/system requests without headers see ALL nodes (426 nodes currently)
- Inconsistent with strict filtering in entities.rs
- Legacy nodes (NULL tenant_id) visible to all

**Inconsistency**: We **just fixed** this exact pattern in `entities.rs` and `relationships.rs` (commit d11edba8), but `graph.rs` still uses the old logic.

---

### 5. Document Listing Permissive Filtering (P1 - HIGH)

**File**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`  
**Function**: `list_documents()` (line 1464-1482)  
**Endpoint**: `GET /api/v1/documents`

**Issue**: Uses permissive filtering - allows documents when tenant context is None.

**Code**:

```rust
let matches_tenant_context = |meta: &DocMetadata| -> bool {
    // If filter_workspace_id is set, document must match
    if let Some(ref filter_ws) = filter_workspace_id {
        if meta.workspace_id.as_ref() != Some(filter_ws) {
            return false;
        }
    }
    // ❌ If filter_workspace_id is None, no filtering happens!

    // If filter_tenant_id is set, document must match
    if let Some(ref filter_tid) = filter_tenant_id {
        if meta.tenant_id.as_ref() != Some(filter_tid) {
            return false;
        }
    }
    // ❌ If filter_tenant_id is None, no filtering happens!

    true
};
```

**Impact**:

- Admin/system requests without headers see ALL documents
- Inconsistent with strict filtering in entities/relationships
- Document metadata leakage across tenants

---

## Secure Endpoints (Verified)

### ✅ Query Execution (query.rs)

**Function**: `execute_query()` (line 100-250)  
**Status**: **SECURE** - Tenant context properly added to engine request.

**Code**:

```rust
if let Some(ref tenant_id) = data_tenant_id {
    engine_request = engine_request.with_tenant_id(tenant_id.clone());
}
if let Some(ref workspace_id) = tenant_ctx.workspace_id {
    engine_request = engine_request.with_workspace_id(workspace_id.clone());
}
```

**Note**: Uses workspace's tenant_id (from database) rather than header tenant_id for data queries (OODA-231.1 fix).

### ✅ Document Upload (documents.rs)

**Function**: `get_workspace_vector_storage_strict()` (line 96-170)  
**Status**: **SECURE** - Strict workspace isolation enforced.

**Code**:

```rust
// OODA-223: NO fallback to default storage in production
// Fails loudly if workspace not found (prevent data isolation bugs)
```

### ✅ Entity/Relationship Handlers (entities.rs, relationships.rs)

**Functions**: `filter_nodes_by_tenant_context()`, `filter_edges_by_tenant_context()`  
**Status**: **SECURE** - Zero-exception filtering enforced (commit d11edba8).

**Code**:

```rust
// SECURITY: STRICT TENANT CONTEXT REQUIRED - NO EXCEPTIONS
if ctx.tenant_id.is_none() || ctx.workspace_id.is_none() {
    tracing::warn!("Tenant context missing - returning empty");
    return Vec::new();
}
```

---

## Security Model Comparison

| Endpoint                      | Tenant Context | Filtering Logic                                    | Status             |
| ----------------------------- | -------------- | -------------------------------------------------- | ------------------ |
| `/api/v1/graph/entities`      | ✅ Required    | **Strict (OR)** - Both tenant & workspace required | ✅ Secure          |
| `/api/v1/graph/relationships` | ✅ Required    | **Strict (OR)** - Both tenant & workspace required | ✅ Secure          |
| `/api/v1/query`               | ✅ Required    | **Strict** - Workspace-based data tenant ID        | ✅ Secure          |
| `/api/v1/documents` (upload)  | ✅ Required    | **Strict** - Workspace vector storage isolation    | ✅ Secure          |
| `/api/v1/documents` (list)    | ✅ Present     | **Permissive (AND)** - Allows None context         | ⚠️ Inconsistent    |
| `/api/v1/graph`               | ✅ Present     | **Permissive** - Allows None context               | ⚠️ Inconsistent    |
| `/api/v1/costs/summary`       | ❌ **MISSING** | **None** - No filtering                            | 🚨 **CRITICAL**    |
| `/api/v1/costs/history`       | ❌ **MISSING** | **None** - No filtering                            | 🚨 **CRITICAL**    |
| `/api/v1/costs/budget`        | ❌ **MISSING** | **None** - Dummy data                              | ⚠️ **Future Risk** |

---

## Recommended Fixes

### Priority 1: Cost Endpoints (P0 - CRITICAL)

**Required Changes**:

1. Add `tenant_ctx: TenantContext` parameter to:
   - `get_cost_summary()`
   - `get_cost_history()`
   - `get_budget_status()`
   - `update_budget()`

2. Filter metadata by tenant context:

   ```rust
   // Only process documents matching tenant context
   if let Some(obj) = value.as_object() {
       let doc_tenant_id = obj.get("tenant_id").and_then(|v| v.as_str());
       let doc_workspace_id = obj.get("workspace_id").and_then(|v| v.as_str());

       // STRICT: Require both tenant_id AND workspace_id to match
       if tenant_ctx.tenant_id.as_deref() != doc_tenant_id {
           continue; // Skip document from other tenant
       }
       if tenant_ctx.workspace_id.as_deref() != doc_workspace_id {
           continue; // Skip document from other workspace
       }

       // ... process document
   }
   ```

3. Add security warning logs when tenant context missing.

### Priority 2: Graph Visualization (P0 - CRITICAL)

**Required Changes**:

1. Update `get_graph()` filtering logic to match `entities.rs`:

   ```rust
   let matches_tenant_context = |properties: &HashMap<String, Value>| {
       // STRICT: Reject if EITHER tenant_id OR workspace_id is missing
       if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
           tracing::warn!("Tenant context missing - returning empty for security");
           return false;
       }

       // Both must match
       let node_tenant_id = properties.get("tenant_id").and_then(|v| v.as_str());
       let node_workspace_id = properties.get("workspace_id").and_then(|v| v.as_str());

       tenant_ctx.tenant_id.as_deref() == node_tenant_id
           && tenant_ctx.workspace_id.as_deref() == node_workspace_id
   };
   ```

2. Remove backward compatibility comment and legacy node support.

### Priority 3: Document Listing (P1 - HIGH)

**Required Changes**:

1. Update `list_documents()` filtering to strict mode:

   ```rust
   let matches_tenant_context = |meta: &DocMetadata| -> bool {
       // STRICT: Both must be present
       if filter_workspace_id.is_none() || filter_tenant_id.is_none() {
           tracing::warn!("Tenant context missing - skipping document");
           return false;
       }

       // Both must match
       meta.workspace_id.as_ref() == filter_workspace_id.as_ref()
           && meta.tenant_id.as_ref() == filter_tenant_id.as_ref()
   };
   ```

2. Add early return if tenant context is None:
   ```rust
   if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
       tracing::warn!("Tenant context missing - returning empty document list");
       return Ok(Json(ListDocumentsResponse {
           documents: vec![],
           total: 0,
       }));
   }
   ```

---

## Testing Plan

### E2E Verification Tests

1. **Cost Leakage Test**:

   ```bash
   # TenantA uploads document with $0.05 cost
   curl -X POST http://localhost:8080/api/v1/documents \
     -H "X-Tenant-ID: TenantA-UUID" \
     -H "X-Workspace-ID: WorkspaceA-UUID" \
     -d '{"text": "test"}'

   # TenantB checks cost summary
   curl http://localhost:8080/api/v1/costs/summary \
     -H "X-Tenant-ID: TenantB-UUID" \
     -H "X-Workspace-ID: WorkspaceB-UUID"

   # EXPECTED: 0 documents, $0.00 cost (isolated)
   # ACTUAL (broken): 1 document, $0.05 cost (LEAKAGE)
   ```

2. **Graph Leakage Test**:

   ```bash
   # Admin (no headers) requests graph
   curl http://localhost:8080/api/v1/graph

   # EXPECTED: 0 nodes (strict enforcement)
   # ACTUAL (broken): 426 nodes (LEAKAGE)
   ```

3. **Document Listing Test**:

   ```bash
   # Admin (no headers) lists documents
   curl http://localhost:8080/api/v1/documents

   # EXPECTED: 0 documents (strict enforcement)
   # ACTUAL (broken): ALL documents (LEAKAGE)
   ```

---

## Commits to Make

1. `security: Fix cost endpoints tenant isolation (CRITICAL)`
2. `security: Enforce strict tenant filtering in graph.rs`
3. `security: Make document listing use strict tenant filtering`
4. `test: Add E2E tests for cost leakage prevention`

---

## Related Commits

- **d11edba8** - `security: Enforce strict tenant context at every layer - NO EXCEPTIONS`
  - Fixed entities.rs and relationships.rs
  - Need to apply same fix to graph.rs and documents.rs
  - Need to add tenant context to costs.rs

---

## Impact Assessment

**Without Fixes**:

- ❌ TenantA can see TenantB's financial costs
- ❌ TenantA can see TenantB's documents
- ❌ TenantA can see TenantB's knowledge graph
- ❌ Admin can bypass tenant isolation
- ❌ Multi-tenant SaaS deployment is **NOT PRODUCTION READY**

**With Fixes**:

- ✅ Perfect tenant isolation across ALL endpoints
- ✅ Consistent strict filtering everywhere
- ✅ Zero data leakage between tenants
- ✅ Production-ready multi-tenant SaaS

---

## Conclusion

Found **5 critical security vulnerabilities** that allow cross-tenant data access. The cost endpoints (`/api/v1/costs/summary`, `/api/v1/costs/history`) are **CRITICAL P0** - they have NO tenant filtering whatsoever.

The graph and document listing endpoints use **permissive filtering** (inconsistent with the strict filtering we just implemented in entities/relationships handlers).

**All issues must be fixed before production deployment of multi-tenant mode.**

---

**Next Steps**: Fix all 5 vulnerabilities using the strict tenant filtering pattern from commit d11edba8.
