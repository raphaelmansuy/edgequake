# 03 — Data Consistency Risks

> **Spec**: 021-storage-study  
> **File**: 05-risks/03-data-consistency-risks.md  
> **Date**: 2026-06-25

---

## R-CONS-01 — Partial Write Window (Vector Store vs AGE Graph)

### Description
Ingestion writes to **two independent storage backends** (vector store and AGE graph)
without a distributed transaction. A crash between Stage 4 (vector write) and
Stage 6 (graph write) leaves orphaned vectors that have no corresponding graph nodes.

### SAGA Compensation
The code implements a forward-compensating SAGA:
```
[write vectors] -> [write KV] -> [write graph]
                                      |
                                  FAILURE?
                                      |
                            [delete vectors for doc]
                            [delete KV keys for doc]
```

### Gaps in the SAGA

| Gap                         | Description                                                                                                                                                  |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| KV write failure            | If KV write (Stage 5) fails AFTER vector write (Stage 4), the SAGA compensates both. ✓                                                                       |
| Graph write partial failure | If the graph MERGE starts but crashes mid-batch (500 nodes/chunk), partial graph writes remain. The SAGA deletes all vectors but leaves partial graph state. |
| Process kill (SIGKILL)      | If the Tokio task is killed after vector write but before SAGA cleanup, vectors orphan permanently.                                                          |
| Concurrent deletion         | If a DELETE request arrives during ingestion, both the task processor and the delete handler may attempt to modify the same vectors/graph entries.           |

### Orphan Detection
There is currently **no periodic invariant checker** to find orphaned vectors
(vectors with `document_id` pointing to a non-existent or failed document).

### Recommendation
1. Add a background task (or CI job): `SELECT DISTINCT document_id FROM eq_*_vectors v WHERE NOT EXISTS (SELECT 1 FROM documents d WHERE d.id::text = v.document_id AND d.status = 'indexed')` → log or delete orphans.
2. Add a `pending_cleanup` flag to `documents` that the SAGA sets before starting compensation, ensuring idempotent cleanup on restart.

---

## R-CONS-02 — No Cross-Store Transaction on Delete

### Description
Document deletion must coordinate across:
1. `documents` table (SQL DELETE)
2. `eq_*_vectors` (delete chunk + entity vectors)
3. `eq_*_kv` (delete chunk text + metadata)
4. AGE graph (DETACH DELETE nodes/edges)

These four operations run as **sequential independent calls**, not in a transaction.

### Failure Scenarios

| Failure Point                                | Result                                          |
| -------------------------------------------- | ----------------------------------------------- |
| After documents DELETE, before vector delete | Orphaned vectors referencing a deleted document |
| After vector delete, before KV delete        | Orphaned KV entries with no document            |
| After KV delete, before graph delete         | Orphaned graph nodes pointing to deleted chunks |
| After graph node delete, before edge cleanup | AGE edges with dangling node references         |

### Current Mitigation
The deletion is wrapped in an application-level retry loop (within the task processor).
AGE `DETACH DELETE` handles edge cascade within the graph, but not across stores.

### Recommendation
1. Mark `documents.status = 'deleting'` atomically BEFORE starting cross-store cleanup.
2. On any failure, the document remains in `deleting` state and a background cleaner retries.
3. Never hard-delete a document record until ALL cross-store cleanup is confirmed.

---

## R-CONS-03 — pdf_documents.document_id Nullable During Processing

### Description
`pdf_documents.document_id` is `NULLABLE` and `NULL` until vision extraction completes
and the document is indexed. This creates a temporary inconsistency window:

```
pdf_documents created   (document_id = NULL)
        |
        v  [vision extraction, may take minutes]
        |
documents record created  (status = indexed)
        |
        v
pdf_documents.document_id = <documents.id>
```

### Risk
During the processing window:
- A query for "all PDFs in workspace" returns PDFs with `document_id = NULL`.
- If the extraction fails and is retried, the `document_id` may be set to a stale or re-created document ID.
- A DELETE for the workspace deletes `pdf_documents` but if `document_id` has been set, also cascades to `documents`. If it hasn't been set, `pdf_documents` is orphaned.

### Recommendation
Ensure `document_id` is set to a pre-allocated UUID at the time `pdf_documents` is created (not after extraction), and the `documents` record is created simultaneously with `processing_status = 'pending'`.

---

## R-CONS-04 — AGE Graph Has No Row-Level Security

### Description
Unlike the relational tables (which use PostgreSQL RLS with `app.tenant_id` GUC),
Apache AGE does not support RLS. Tenant isolation in the graph is enforced purely
by **application-level property filtering**:

```cypher
MATCH (n:Node)
WHERE n.tenant_id = 'tenant-uuid' AND n.workspace_id = 'ws-uuid'
RETURN n
```

If a query path **omits** the tenant/workspace filter, it will return all nodes from
all tenants.

### Evidence
In `edgequake-storage/src/adapters/postgres/graph/nodes_ops.rs`, most query methods
accept `tenant_id: Option<&str>` and `workspace_id: Option<&str>`. If the caller
passes `None`, no isolation is enforced:
```rust
pub async fn search_nodes(
    &self,
    query: &str,
    limit: usize,
    entity_type: Option<&str>,
    tenant_id: Option<&str>,    // <-- None = all tenants!
    workspace_id: Option<&str>, // <-- None = all workspaces!
) -> Result<Vec<(GraphNode, usize)>>
```

### Recommendation
1. In production, **always pass** tenant_id and workspace_id from `TenantContext` middleware.
2. Consider making tenant_id/workspace_id **required** parameters (not `Option`) in graph read methods, with explicit `ALL_TENANTS` sentinel for admin-only code.
3. Add integration tests that verify cross-tenant data isolation in graph queries.
