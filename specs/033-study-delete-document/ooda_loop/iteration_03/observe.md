# ITERATION 03 - OBSERVE

**Date**: 2026-01-26  
**Mission**: Study document add/delete process on EdgeQuake (specs/033-study-delete-document/003-study-document.md)  
**Focus**: PostgreSQL provider verification + reprocessing mechanism review

---

## Mission Re-Read ✅

From mission file:

> "Ensure it working with postgres provider and memory provider for all storage layers (KV, Vector, Graph)."
> "Ensure there is reprocessing mechanism for failed documents."
> "Ensure deleting a failed document cleans up all partial data."

---

## OBSERVATION 1: PostgreSQL Graph Storage Implementation

### Location

`edgequake/crates/edgequake-storage/src/adapters/postgres/graph.rs`

### Key Findings

**Node Deletion**:

```rust
// Line 785-796
async fn delete_node(&self, node_id: &str) -> Result<()> {
    let escaped_id = Self::escape_cypher_string(node_id);

    // Use DETACH DELETE to remove node and all connected edges
    let cypher = format!(
        "MATCH (n:Node {{node_id: '{}'}}) DETACH DELETE n",
        escaped_id
    );

    self.cypher_execute(&cypher).await
}
```

**Analysis**:

- Uses Apache AGE with Cypher query language
- `DETACH DELETE` removes node AND all connected edges atomically
- This is actually safer than our explicit edge cleanup in the API layer
- WHY: AGE guarantees no orphaned edges when using DETACH DELETE

**Edge Deletion**:

```rust
// Line 870-889
async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
    let escaped_source = Self::escape_cypher_string(source);
    let escaped_target = Self::escape_cypher_string(target);

    let cypher = format!(
        "MATCH (s:Node {{node_id: '{}'}})-[r:Relationship]->(t:Node {{node_id: '{}'}}) DELETE r",
        escaped_source, escaped_target
    );

    self.cypher_execute(&cypher).await
}
```

**Analysis**:

- Deletes specific relationship between two nodes
- Safe: Only deletes if both nodes exist
- No orphan risk: Edge can't exist without endpoints

---

## OBSERVATION 2: Memory Graph Storage Implementation

### Location

`edgequake/crates/edgequake-storage/src/adapters/memory/graph.rs`

### Key Findings

**Node Deletion**:

```rust
// Uses HashMap<String, GraphNode> - O(1) deletion
async fn delete_node(&self, node_id: &str) -> Result<()> {
    self.nodes.lock().await.remove(node_id);
    Ok(())
}
```

**Edge Deletion**:

```rust
async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
    let mut edges = self.edges.lock().await;
    edges.retain(|e| !(e.source == source && e.target == target));
    Ok(())
}
```

**Analysis**:

- Simple in-memory storage with Mutex-protected HashMaps
- No automatic cascade: edges can become orphaned if node deleted first
- WHY our API-level orphan detection is important for Memory provider

---

## OBSERVATION 3: Reprocessing Mechanism

### Endpoints Available

**1. Reprocess Failed Documents**

- **Endpoint**: `POST /api/v1/documents/reprocess`
- **Location**: documents.rs:2842
- **Request**: `ReprocessFailedRequest { max_documents, track_id }`
- **Behavior**:
  1. Find documents with status="failed"
  2. Update status to "pending"
  3. Set new track_id and retry_at timestamp
  4. Create new processing task
  5. Queue task for background processing

**2. Recover Stuck Documents**

- **Endpoint**: `POST /api/v1/documents/recover-stuck`
- **Location**: documents.rs:2984
- **Request**: `RecoverStuckRequest { stuck_threshold_minutes, max_documents, document_ids }`
- **Behavior**:
  1. Find documents with status="processing" older than threshold
  2. Update status to "pending"
  3. Set new track_id and recovered_at timestamp
  4. Create new processing task
  5. Queue task for background processing

### Current Gap

**ISSUE**: Reprocessing does NOT clean up partial data from failed attempt first!

**Problem Scenario**:

```
T0: Document uploaded (async)
T1: Processing starts, creates entities A, B
T2: Processing fails (LLM timeout, OOM, etc.)
    → Status set to "failed"
    → Entities A, B still exist in graph
    → Embeddings for A, B still exist in vector storage

T3: User calls reprocess_failed
    → Status set to "pending"
    → New task created
    → Processing starts again

T4: Processing creates entities A, B, C
    → A and B already exist → DUPLICATE entities!
    → Or: upsert updates existing → inconsistent source_ids
```

**Root Cause**:

- `reprocess_failed` doesn't call `delete_document` first
- Partial data accumulates on retry
- Entity deduplication relies on LLM to use same names (not guaranteed)

---

## OBSERVATION 4: Partial Data Cleanup Verification

### Test: Deleting Failed Document

**Current Test**: `test_delete_failed_document_allowed` (e2e_document_deletion.rs:579)

```rust
// Creates document with status="failed" manually
// Verifies deletion returns 200 OK
```

**Gap**: This test doesn't verify partial entities/edges are cleaned up!

**What We Need to Test**:

1. Create document that fails mid-processing (has some entities)
2. Delete the failed document
3. Verify ALL partial entities are removed
4. Verify ALL partial edges are removed
5. Verify ALL partial embeddings are removed

---

## OBSERVATION 5: Storage Provider Consistency

### Deletion Behavior Comparison

| Operation     | Memory Provider       | PostgreSQL Provider  |
| ------------- | --------------------- | -------------------- |
| Delete Node   | Removes from HashMap  | Cypher DETACH DELETE |
| Cascade Edges | NO (manual needed)    | YES (automatic)      |
| Delete Edge   | Removes from Vec      | Cypher DELETE        |
| Transaction   | NO (each op separate) | YES (within query)   |
| Orphan Risk   | YES                   | NO                   |

### Implications

1. **Memory Provider**: Our API-level orphan detection (ITERATION 01) is CRITICAL
2. **PostgreSQL Provider**: DETACH DELETE handles cascade, but our API logic still works (harmless redundancy)
3. **Both**: Status check (ITERATION 02) provides consistent safety

---

## OBSERVATION 6: Missing Test Scenarios

### Edge Cases Not Yet Tested

| Scenario                                      | Priority | Status        |
| --------------------------------------------- | -------- | ------------- |
| Delete document with partial entities         | HIGH     | ❌ Not tested |
| Delete document, then reprocess sibling       | HIGH     | ❌ Not tested |
| Reprocess failed document cleans partial data | HIGH     | ❌ Not tested |
| PostgreSQL cascade behavior                   | MEDIUM   | ❌ Not tested |
| Memory vs PostgreSQL consistency              | MEDIUM   | ❌ Not tested |
| Concurrent deletion of same document          | LOW      | ❌ Not tested |

---

## Summary

### Already Working ✅

1. **Status-based deletion safety** (ITERATION 02)
   - "pending" → 409 Conflict
   - "processing" → 409 Conflict
   - "failed" → 200 OK (delete allowed)

2. **Reference counting** (ITERATION 01)
   - Entities/edges check source_ids before deletion
   - Shared entities preserved

3. **Reprocessing endpoints exist**
   - `/api/v1/documents/reprocess`
   - `/api/v1/documents/recover-stuck`

4. **PostgreSQL graph storage**
   - DETACH DELETE provides atomic cascade

### Gaps Found ❌

1. **Reprocessing doesn't clean partial data first**
   - Can cause duplicate/inconsistent entities

2. **No tests for partial data cleanup**
   - Can't prove failed document deletion cleans all data

3. **No PostgreSQL-specific integration tests**
   - Status check works with memory, not verified with PostgreSQL

---

## Evidence References

| File                     | Lines     | Description                         |
| ------------------------ | --------- | ----------------------------------- |
| postgres/graph.rs        | 785-796   | delete_node with DETACH DELETE      |
| postgres/graph.rs        | 870-889   | delete_edge Cypher query            |
| memory/graph.rs          | -         | Simple HashMap deletion             |
| documents.rs             | 2842-2963 | reprocess_failed endpoint           |
| documents.rs             | 2984-3100 | recover_stuck endpoint              |
| e2e_document_deletion.rs | 579-628   | test_delete_failed_document_allowed |

---

**Status**: OBSERVE COMPLETE ✅  
**Next**: Create ORIENT to analyze solutions for gaps
