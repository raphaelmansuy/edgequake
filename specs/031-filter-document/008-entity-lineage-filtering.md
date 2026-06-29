# SPEC-031 / 008 — Entity & Relationship Scope Filtering via Lineage

> **Lens**: LightRAG AI Engineer · System Engineer
> **Status**: Implementation complete
> **Cross-refs**: SPEC-031/001-007, `context_filter.rs`, `helpers.rs`, `modes/`

---

## 1. Query Pipeline with Document Scope — Full ASCII Diagram

```
POST /api/v1/query  {document_filter: {document_ids: ["A","B"]}}
       |
       v
  document_filter_resolver.rs
  (resolves patterns/dates → Vec<String> of doc IDs)
       |
       v  allowed_document_ids = ["A","B"]
  query_pipeline.rs  pipeline_retrieve()
       |
       +-- passed as allowed_document_ids to mode functions
       |
       v
  ┌─────────────────────────────────────────────────────────────────────┐
  │  RETRIEVAL PHASE  (per mode)                                        │
  │                                                                     │
  │  1. VECTOR PRE-FILTER (SQL layer — fast, before any results return) │
  │     MetadataFilter {                                                │
  │       tenant_id, workspace_id,                                      │
  │       document_ids: Some(["A","B"])   ← NEW: SPEC-031 Tier 1       │
  │     }                                                               │
  │     SQL: WHERE metadata->>'source_document_id' = ANY('{A,B}')      │
  │     → Only entity/chunk vectors from docs A,B returned from DB     │
  │                                                                     │
  │  2. GRAPH TRAVERSAL (for local/global modes)                       │
  │     entity_ids from vector results                                  │
  │     → get_nodes_batch(entity_ids)                                  │
  │     → edges_within_depth(entity_ids, depth=2)                      │
  │     Traversal is unscoped (cross-doc graph is intentional)         │
  │     → Filter applied POST-traversal in context_filter.rs           │
  └─────────────────────────────────────────────────────────────────────┘
       |
       v
  enrich_retrieved_context()
       |
       v
  ┌─────────────────────────────────────────────────────────────────────┐
  │  POST-RETRIEVAL FILTER  context_filter.rs  (SPEC-031 Tier 2)       │
  │                                                                     │
  │  Chunks (STRICT):                                                   │
  │    document_id in allowed_ids?  → keep                             │
  │    document_id absent/wrong?    → exclude                          │
  │                                                                     │
  │  Entities (STRICT when filter active):                             │
  │    source_document_ids[] ∩ allowed_ids ≠ ∅ → keep                 │
  │    source_document_id in allowed_ids       → keep (fallback)       │
  │    NO lineage data at all                  → keep (unknown prov.)  │
  │    lineage data exists but doesn't match   → EXCLUDE ← FIX        │
  │                                                                     │
  │  Relationships (same rule as entities)                              │
  └─────────────────────────────────────────────────────────────────────┘
       |
       v
  Rerank → Sort → Truncate → LLM → Response
```

---

## 2. Current Gaps (Code is Law Audit)

### GAP-1: Entity filter is lenient — provenance-free entities bypass scope

**File**: `context_filter.rs`

```rust
// CURRENT (lenient — BUG when scope is active):
context.entities.retain(|entity| {
    entity.source_document_id.as_deref()
        .map(|id| id_set.contains(id))
        .unwrap_or(true)  // ← keeps entities with NO source_document_id
});
```

**Problem**: Any entity without `source_document_id` (including entities whose
provenance was dropped during reconciliation) bypasses the scope filter.

**Fix**: Also check `source_document_ids` array. Only use the lenient fallback
(`unwrap_or(true)`) when the entity has ABSOLUTELY no lineage data at all.

---

### GAP-2: Multi-document entity lineage is a single string

**File**: `entity_reconcile.rs`, `merger/metadata.rs`

During ingestion, when entity "JOHN_DOE" is found in both doc-A and doc-B:
- `source_chunk_ids` → correctly UNIONED: `["chunk-1", "chunk-3"]`
- `source_document_id` → OVERWRITTEN: only holds the last-processed doc ID

When querying with `allowed_document_ids = ["doc-A"]` and JOHN_DOE's
`source_document_id` is "doc-B", JOHN_DOE is incorrectly excluded.

**Fix**: During reconciliation, union `source_document_id` values into a
`source_document_ids` JSON array (same pattern as `source_chunk_ids`).
Set `source_document_ids` on every newly ingested entity.

**Note**: Existing stored entities won't have `source_document_ids` until
re-processed. The context_filter falls back to `source_document_id` (single)
and finally to `unwrap_or(true)` for true no-provenance entities.

---

### GAP-3: Vector pre-filter never uses `allowed_document_ids`

**Files**: `modes/local.rs`, `modes/global.rs`, `modes/naive.rs`

All three modes construct `MetadataFilter` without `document_ids`:

```rust
// CURRENT:
let mf = MetadataFilter::from_tenant_workspace(tenant_id.clone(), workspace_id.clone());
```

`MetadataFilter.document_ids` EXISTS in the struct and IS handled in SQL
(`WHERE metadata->>'source_document_id' = ANY($1::text[])`) but is NEVER
populated from the query request's `allowed_document_ids`.

**Fix**: Pass `allowed_document_ids` into mode functions and populate
`MetadataFilter.document_ids`. This pushes the scope filter to the SQL layer
(Tier 1 pre-filter) for maximum efficiency.

---

### GAP-4: Graph traversal is fully unscoped

**Files**: `graph_hops.rs`, `modes/local.rs`, `modes/global.rs`

After finding seed entities from vector search, graph traversal
(`edges_within_depth`) fetches ALL related entities/relationships regardless
of their document scope. These traversal results bypass the vector pre-filter.

**Mitigation**: The post-retrieval `context_filter.rs` handles this correctly
(after the Tier 1 gap fix). Full graph-level filtering would require passing
`allowed_document_ids` into graph storage queries — this is documented as
a future enhancement (Tier 3).

---

## 3. `RetrievedEntity` and `RetrievedRelationship` Extension

### 3.1 New Fields

Add `source_document_ids: Vec<String>` to both structs in `context.rs`:

```rust
pub struct RetrievedEntity {
    // ... existing fields ...
    pub source_document_id: Option<String>,   // single (backward compat)
    pub source_document_ids: Vec<String>,     // NEW: union of all docs ← SPEC-031
    pub source_file_path: Option<String>,
}

pub struct RetrievedRelationship {
    // ... existing fields ...
    pub source_document_id: Option<String>,   // single (backward compat)
    pub source_document_ids: Vec<String>,     // NEW: union of all docs ← SPEC-031
    pub source_file_path: Option<String>,
}
```

### 3.2 `helpers.rs` — Read `source_document_ids` Array

```rust
pub fn extract_entity_source_tracking(props: &HashMap<String, Value>) -> EntitySourceTracking {
    let source_document_ids: Vec<String> = props
        .get("source_document_ids")           // NEW plural field
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let source_document_id = props
        .get("source_document_id")            // legacy single field
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // ...
}
```

---

## 4. `context_filter.rs` — Strict Entity/Relationship Filter

### 4.1 New Logic

```
For each entity E, given allowed_id_set S:

  1. If source_document_ids (plural) is non-empty:
     → Keep if source_document_ids ∩ S ≠ ∅
     → Exclude otherwise

  2. Else if source_document_id (singular) is Some(id):
     → Keep if id ∈ S
     → Exclude otherwise

  3. Else (no lineage data at all):
     → Keep (truly unknown provenance — could be globally derived entity)
```

This is STRICT: once ANY lineage data is present, it must match.
Only entities with zero lineage data bypass the filter.

---

## 5. Vector Pre-Filter Integration

### 5.1 `QueryRequest` change

`QueryRequest` in `edgequake-query/src/types.rs` already carries
`allowed_document_ids: Option<Vec<String>>`. This needs to be threaded
through `pipeline_retrieve()` into the mode functions.

### 5.2 Mode function signatures (all three)

```rust
// Before:
pub(in crate::engine_impl) async fn query_local_with_vector_storage(
    &self,
    query_text: &str,
    keywords: &ExtractedKeywords,
    embeddings: &QueryEmbeddings,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    vector_storage: &Arc<dyn VectorStorage>,
    max_chunks: usize,
) -> Result<QueryContext>

// After:
pub(in crate::engine_impl) async fn query_local_with_vector_storage(
    &self,
    query_text: &str,
    keywords: &ExtractedKeywords,
    embeddings: &QueryEmbeddings,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    allowed_document_ids: Option<&[String]>,   // ← NEW SPEC-031
    vector_storage: &Arc<dyn VectorStorage>,
    max_chunks: usize,
) -> Result<QueryContext>
```

### 5.3 MetadataFilter construction in modes

```rust
// DRY helper in a shared module:
fn make_metadata_filter(
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    allowed_document_ids: Option<&[String]>,
    vector_type: Option<&str>,
) -> Option<MetadataFilter> {
    let has_filters = tenant_id.is_some()
        || workspace_id.is_some()
        || allowed_document_ids.is_some();
    if !has_filters {
        return None;
    }
    Some(MetadataFilter {
        tenant_id,
        workspace_id,
        document_ids: allowed_document_ids
            .map(|ids| ids.iter().map(String::from).collect()),
        vector_type: vector_type.map(str::to_string),
    })
}
```

---

## 6. Edge Cases

| Scenario | Behavior |
|----------|---------|
| Entity has `source_document_ids = []` empty | Falls back to `source_document_id` check |
| Entity has `source_document_id = None` AND `source_document_ids = []` | Kept (no provenance data — global entity) |
| Entity appears in docs A+B, scope = [A] | Kept (A ∈ source_document_ids) |
| Entity appears in docs A+B, scope = [C] | Excluded (C ∉ source_document_ids) |
| Vector pre-filter with `source_document_ids` array | SQL only checks `source_document_id` (single) — multi-doc entities may not be found via pre-filter; post-filter is the safety net |
| `allowed_document_ids = []` | Empty filter → returns `Some([])` → all chunks excluded, lenient entities kept |
| No scope filter | `None` → all modes unchanged, zero overhead |

---

## 7. Implementation Checklist

- [x] `RetrievedEntity.source_document_ids: Vec<String>` added (context.rs)
- [x] `RetrievedRelationship.source_document_ids: Vec<String>` added (context.rs)
- [x] `helpers.rs` reads `source_document_ids` array from node/edge properties
- [x] `context_filter.rs` strict filter using source_document_ids (plural first, singular fallback)
- [x] `make_scope_metadata_filter()` DRY helper in `modes/mod.rs`
- [x] `local.rs` mode: passes `allowed_document_ids` to `MetadataFilter`
- [x] `global.rs` mode: passes `allowed_document_ids` to `MetadataFilter`
- [x] `naive.rs` mode: passes `allowed_document_ids` to `MetadataFilter`
- [x] `hybrid.rs`, `mix.rs`: propagate new parameter
- [x] `query_pipeline.rs`: passes `request.allowed_document_ids` into mode calls
- [x] `query_modes.rs`: passes `allowed_document_ids` into mode calls
- [ ] `entity_reconcile.rs`: union `source_document_id` into `source_document_ids` array (Phase 2)
- [ ] `metadata_filter_sql.rs`: add `source_document_ids` array matching in SQL (Phase 2)
- [x] Backend unit tests for all new filter behaviors
- [x] E2E test: scope query proves only scoped document entities in context
