# SPEC-034-008: Implementation Guide — Code Changes

> **Lens**: Rust / LightRAG Expert  
> **Version**: 1.0.0 — 2026-06-30

---

## 1. Architecture of the Native SQL Write Path (IMP-01)

```
CURRENT ARCHITECTURE:
┌─────────────────────────────────────────────────────────────────────────────┐
│  KnowledgeGraphMerger                                                       │
│    → merge_entities_batch()                                                 │
│      → graph_storage.upsert_nodes_batch(&nodes)                             │
│        → PostgresAGEGraphStorage::pg_upsert_nodes_batch()                  │
│          → cypher_execute("UNWIND [...] MERGE (n:Node {...}) SET n.k=v.k")  │
│            [AGE GIN lookup: O(G) per node]                                  │
└─────────────────────────────────────────────────────────────────────────────┘

TARGET ARCHITECTURE:
┌─────────────────────────────────────────────────────────────────────────────┐
│  KnowledgeGraphMerger                                                       │
│    → merge_entities_batch()                                                 │
│      → graph_storage.upsert_nodes_batch(&nodes)                             │
│        → PostgresAGEGraphStorage::pg_upsert_nodes_batch()                  │
│          [if NATIVE_GRAPH_WRITES enabled:]                                  │
│          → pg_upsert_nodes_batch_native()                                   │
│            → SQL: INSERT INTO "graph"."Node" (id, properties)               │
│                   SELECT eq_next_node_id(...), build_agtype(...)             │
│                   FROM unnest($ids, $props_json)                             │
│                   ON CONFLICT (node_id_expr) DO UPDATE SET properties = ... │
│                   [btree index: O(log G) per node]                          │
│          [else: Cypher UNWIND MERGE — existing path]                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Key Implementation Challenge: AGE agtype Format

Apache AGE stores properties as `agtype` — a PostgreSQL extension of JSONB.
To write natively, we must produce valid agtype from Rust.

```
agtype binary format:
  - Identical to JSONB for simple types (strings, numbers, booleans, null)
  - Extended with: graphid, vertex, edge types
  - A valid agtype for properties is: JSON object cast to agtype

Cast: '{"node_id": "X", "entity_type": "PERSON"}'::jsonb::agtype
```

**Implementation**:

```rust
// In cypher_format.rs — add agtype serializer
pub fn properties_to_agtype_literal(
    props: &HashMap<String, serde_json::Value>
) -> String {
    // For write path: serialize as JSON, cast to agtype in SQL
    // The cast `::jsonb::agtype` handles type conversion server-side
    serde_json::to_string(props).unwrap_or_default()
}
```

---

## 3. Native SQL Node Upsert — Rust Implementation

**File**: `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/nodes_ops.rs`

```rust
impl PostgresAGEGraphStorage {
    /// IMP-01: Native SQL node batch upsert — O(log G) per node.
    ///
    /// # WHY: Replace Cypher MERGE with native SQL INSERT ON CONFLICT
    ///
    /// AGE's cypher() compiles {node_id: 'X'} to GIN @> containment lookup.
    /// Native SQL uses the btree index idx_node_prop_node_id_btree directly.
    /// This changes complexity from O(G) to O(log G) per node — ~69× faster.
    ///
    /// # AGE Compatibility
    ///
    /// AGE stores nodes in "{graph}"."Node" with:
    ///   id: graphid (int64 = label_oid << 32 | sequence)
    ///   properties: agtype (binary-compatible with jsonb for simple types)
    ///
    /// We generate valid graphids using the SQL function eq_next_node_id()
    /// (migration 067) which calls nextval on the AGE sequence.
    ///
    /// # Conflict Resolution
    ///
    /// ON CONFLICT ON EXPRESSION (agtype_to_json(properties)->>'node_id')
    /// matches the btree index added in SPEC-032.
    /// DO UPDATE SET properties = merges old + new properties server-side.
    pub(super) async fn pg_upsert_nodes_batch_native(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        let pool = self.pool.get().await?;
        let graph = &self.graph_name;

        // Serialize properties as JSONB strings
        let node_ids: Vec<String> = nodes.iter().map(|(id, _)| id.clone()).collect();
        let props_json: Vec<String> = nodes
            .iter()
            .map(|(id, props)| {
                let mut full = props.clone();
                full.insert("node_id".into(), serde_json::Value::String(id.clone()));
                serde_json::to_string(&full).unwrap_or_else(|_| "{}".to_string())
            })
            .collect();

        // Build the batch SQL
        // Note: unnest($1::text[], $2::text[]) expands parallel arrays
        let sql = format!(
            r#"
            INSERT INTO {graph}."Node" (id, properties)
            SELECT 
                eq_next_node_id('{graph}'),
                p.props_text::jsonb::agtype
            FROM unnest($1::text[], $2::text[]) AS p(node_id_val, props_text)
            ON CONFLICT ((ag_catalog.agtype_to_json(properties)->>'node_id'))
            DO UPDATE SET 
                properties = EXCLUDED.properties
            WHERE {graph}."Node".id IS NOT NULL
            "#,
            graph = graph
        );

        sqlx::query(&sql)
            .bind(&node_ids)
            .bind(&props_json)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!(
                "Native SQL node batch upsert failed: {}", e
            )))?;

        Ok(())
    }
}
```

**⚠ Note**: The `ON CONFLICT` clause requires a unique constraint or unique index
on the expression `(ag_catalog.agtype_to_json(properties)->>'node_id')`.  
This is added by Migration 072.

---

## 4. Feature Flag Implementation

**File**: `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs`

```rust
/// IMP-01: Use native SQL writes instead of AGE cypher() for upserts.
///
/// Set EDGEQUAKE_NATIVE_GRAPH_WRITES=1 to enable.
/// Default: disabled (false) until fully validated.
pub(super) fn native_graph_writes_enabled() -> bool {
    std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}
```

**In `nodes_ops.rs`** — add dispatch:

```rust
pub(super) async fn pg_upsert_nodes_batch(
    &self,
    nodes: &[(String, HashMap<String, serde_json::Value>)],
) -> Result<()> {
    if nodes.is_empty() {
        return Ok(());
    }

    // IMP-01: Native SQL path (opt-in via env var)
    if native_graph_writes_enabled() {
        return self.pg_upsert_nodes_batch_native(nodes).await;
    }

    // Existing Cypher UNWIND MERGE path (unchanged)
    // ... (existing implementation) ...
}
```

---

## 5. Async Community Indexing (IMP-06)

**File**: `edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs`

```rust
// BEFORE (line ~320):
edgequake_storage::schedule_community_index_refresh(
    graph_storage.clone(),
    ctx.workspace_id.clone(),
).await;

// AFTER: fire-and-forget background task
let gs = graph_storage.clone();
let ws = ctx.workspace_id.clone();
tokio::spawn(async move {
    // WHY: Community index refresh is a read-model rebuild; it doesn't
    // affect the correctness of the current persist. Blocking the persist
    // path on it adds latency with no benefit to the caller.
    edgequake_storage::schedule_community_index_refresh(gs, ws).await;
});
```

---

## 6. KV Store — Disable GIN Maintenance

The GIN index drop (Migration 068) handles the database side.  
No Rust code changes needed — the `value_gin` index name is not referenced
in Rust code (sqlx uses the table structure, not index names).

---

## 7. Performance Benchmarks to Add

**File**: `edgequake/crates/edgequake-storage/tests/perf_graph_write.rs`

```rust
#[cfg(test)]
#[cfg(feature = "integration")]
mod tests {
    use super::*;
    use std::time::Instant;

    /// SPEC-034: Baseline benchmark for node batch upsert
    #[tokio::test]
    async fn bench_node_upsert_batch_200() {
        let storage = create_test_storage().await;
        let nodes = generate_test_nodes(200);
        
        let start = Instant::now();
        storage.upsert_nodes_batch(&nodes).await.unwrap();
        let elapsed = start.elapsed();
        
        println!("Node batch upsert 200 nodes: {:?}", elapsed);
        
        // Acceptance criterion: < 500ms for 200 nodes
        assert!(
            elapsed.as_millis() < 500,
            "Node batch upsert took {}ms — expected < 500ms (SPEC-034 IMP-01 criterion)",
            elapsed.as_millis()
        );
    }
    
    /// SPEC-034: Measure cost at scale (10K existing nodes)
    #[tokio::test]
    async fn bench_node_upsert_at_scale() {
        let storage = create_test_storage().await;
        
        // Pre-populate with 10K nodes
        let existing = generate_test_nodes(10_000);
        storage.upsert_nodes_batch(&existing).await.unwrap();
        
        // Now measure adding 200 more
        let new_nodes = generate_test_nodes_offset(200, 10_000);
        
        let start = Instant::now();
        storage.upsert_nodes_batch(&new_nodes).await.unwrap();
        let elapsed = start.elapsed();
        
        println!("Node batch upsert 200 nodes (into 10K graph): {:?}", elapsed);
        assert!(elapsed.as_millis() < 1_000,
            "Node upsert at scale took {}ms — expected < 1000ms", elapsed.as_millis());
    }
}
```

---

## 8. LightRAG Algorithm Compatibility

The LightRAG entity deduplication algorithm depends on:

1. **Entity normalization** — `EntityId::new(name)` normalizes to uppercase with underscores
2. **Get-then-set merge** — `get_nodes_batch` → compare descriptions → `upsert_nodes_batch`
3. **Conflict semantics** — `ON CONFLICT DO UPDATE` must preserve existing properties and merge new ones

The native SQL path must implement the same merge semantics as Cypher MERGE:

```sql
-- Cypher MERGE semantics:
--   If node exists: SET n.k = new_v for each key in new props
--   If node new: create with all properties

-- SQL equivalent:
ON CONFLICT ((agtype_to_json(properties)->>'node_id'))
DO UPDATE SET properties = (
    -- Merge old and new properties: new values overwrite old
    -- This matches SET n.key = new_value behavior
    ag_catalog.agtype_build_map_noargs() || 
    EXCLUDED.properties
)
```

**Test for correctness**:

```rust
#[tokio::test]
async fn test_native_upsert_preserves_existing_properties() {
    let storage = create_test_storage().await;
    
    // First write: node with description A
    let nodes_v1 = vec![
        ("ENTITY_X".to_string(), map!{"entity_type" => "PERSON", "description" => "Version 1"})
    ];
    storage.upsert_nodes_batch(&nodes_v1).await.unwrap();
    
    // Second write: same node, different description
    let nodes_v2 = vec![
        ("ENTITY_X".to_string(), map!{"entity_type" => "PERSON", "description" => "Version 2"})
    ];
    storage.upsert_nodes_batch(&nodes_v2).await.unwrap();
    
    // Node should exist exactly once, with Version 2 description
    let node = storage.get_node("ENTITY_X").await.unwrap().unwrap();
    assert_eq!(node.properties["description"], "Version 2");
    
    // Count nodes — must be exactly 1 (not 2)
    let count = storage.node_count().await.unwrap();
    assert_eq!(count, 1);
}
```

---

## 9. Monitoring Hooks

Add timing instrumentation to detect regressions:

```rust
// In pg_upsert_nodes_batch():
let start = std::time::Instant::now();
// ... upsert ...
let elapsed = start.elapsed();
if elapsed.as_millis() > 500 {
    tracing::warn!(
        batch_size = nodes.len(),
        elapsed_ms = elapsed.as_millis(),
        "Node batch upsert exceeded 500ms threshold (SPEC-034)"
    );
}
tracing::debug!(
    batch_size = nodes.len(),
    elapsed_ms = elapsed.as_millis(),
    native_path = native_graph_writes_enabled(),
    "Node batch upsert completed"
);
```

---

## 10. Acceptance Test Matrix

| Test                          | Criteria                 | Command                                  |
| ----------------------------- | ------------------------ | ---------------------------------------- |
| Node upsert 200 entities      | < 500ms                  | `cargo test bench_node_upsert_batch_200` |
| Node upsert at 50K graph      | < 1,000ms                | `cargo test bench_node_upsert_at_scale`  |
| Deduplication correctness     | 0 duplicate nodes        | `cargo test test_native_upsert_dedup`    |
| Edge upsert 500 edges         | < 800ms                  | `cargo test bench_edge_upsert_batch_500` |
| All existing tests pass       | 100% pass                | `cargo test --workspace --lib`           |
| No seq scans on indexed reads | EXPLAIN shows Index Scan | `cargo test explain_no_seqscan`          |
| KV upsert without GIN         | < 1ms                    | `cargo test bench_kv_upsert`             |
| Community index async         | Persist < 2s             | `cargo test persist_no_community_block`  |
