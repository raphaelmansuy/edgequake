# SPEC-005: Date & Document Pattern Filters for Queries and Document Listing

> **Issue**: [#75 — Add optional date Filter parameters to a user request](https://github.com/raphaelmansuy/edgequake/issues/75)
> **Status**: Accepted
> **Priority**: High
> **Complexity**: Medium

## Summary

Add optional **date range** and **document pattern** filter parameters to both **RAG queries** and **document listing**. These filters narrow the search scope to only documents matching the criteria, applied as early as architecturally possible in the retrieval pipeline.

### Motivation (from issue)

> "User may want to narrow its search to a specific range of date (only documents created during last months, from date start until date end, before date end). This is a feature expected when dealing with living and complex document databases."

### Extended Scope: Document Pattern Filters

In addition to date filtering, users need to filter by **document name patterns** (glob or substring match on title/file_name). This enables queries like:

- *"Search only in financial reports"* → pattern: `*financial*`
- *"Query my Q1 2025 documents"* → pattern: `Q1-2025*` + date range

---

## Requirements

### FR-001: Date Range Filter for RAG Queries

**As a** user, **I want** to restrict my RAG query to documents within a date range, **so that** I get answers grounded only in recent/relevant documents.

#### Date Range Semantics (from issue)

| Variant | Filter | Description |
|---------|--------|-------------|
| Open Start | `[null, end_date]` | Documents created **on or before** `end_date` |
| Open End | `[start_date, null]` | Documents created **on or after** `start_date` |
| Closed | `[start_date, end_date]` | Documents created **within** the inclusive range |
| No Filter | `[null, null]` | All documents — filter inactive (default) |

- Dates are ISO 8601 format: `YYYY-MM-DDTHH:MM:SSZ` or `YYYY-MM-DD`
- Date comparison uses `created_at` field on document metadata
- Both boundaries are **inclusive**

### FR-002: Document Pattern Filter for RAG Queries

**As a** user, **I want** to restrict my RAG query to documents matching a name pattern, **so that** I focus results on specific document groups.

- Pattern matches against `title`, `file_name`, or `file_path`
- Case-insensitive substring match
- Multiple patterns can be comma-separated (OR logic)
- Example: `financial,report` matches documents containing "financial" OR "report"

### FR-003: Date Range Filter for Document Listing

**As a** user, **I want** to filter the document list by creation date range, **so that** I can quickly find documents from a specific time period.

- Same date semantics as FR-001
- Applied server-side before pagination

### FR-004: Document Pattern Filter for Document Listing

**As a** user, **I want** to search/filter documents by name in the document list, **so that** I can find specific documents quickly.

- Same pattern semantics as FR-002
- Applied server-side before pagination

### FR-005: Client-Side Date Filter UI (Query Page)

**As a** user, **I want** a date range picker in the query interface, **so that** I can easily set temporal boundaries for my queries.

- Two date inputs: "From" and "To"
- Either can be left empty (open interval)
- Quick presets: "Last 7 days", "Last 30 days", "Last 90 days", "This year"

### FR-006: Client-Side Date Filter UI (Documents Page)

**As a** user, **I want** a date range picker in the document manager, **so that** I can browse documents by time period.

- Integrated into the existing filter toolbar
- Same presets as FR-005

### FR-007: Document Pattern Filter UI (Query Page)

**As a** user, **I want** a document filter input in the query interface, **so that** I can target my query to specific documents.

- Text input with placeholder: "Filter documents by name…"
- AutoComplete/suggestions from document list (optional, Phase 2)

---

## Non-Functional Requirements

| ID | Requirement |
|----|-------------|
| NFR-001 | Date filtering must not add > 10ms latency to queries |
| NFR-002 | Pattern matching must be case-insensitive |
| NFR-003 | Filters must compose with existing tenant/workspace isolation |
| NFR-004 | Empty/null filters must be a no-op (backward compatible) |
| NFR-005 | API must validate date format and reject invalid dates with 400 |

---

## Architecture & Design

### Deep Analysis: Existing Pre-Filtering Architecture

A thorough investigation of the EdgeQuake storage layer reveals how tenant/workspace/document isolation is currently implemented across the three storage backends.

#### Storage Layer Inventory

| Storage | Backend | Tenant Isolation Method | Document-Level Filter Support |
|---------|---------|------------------------|-------------------------------|
| **Vector** (`PgVectorStorage`) | PostgreSQL + pgvector | **App-level post-retrieval** via `matches_tenant_filter` on JSONB metadata; 17 call sites in `vector_queries.rs` (6) and `query_modes.rs` (11) | ⚠️ `document_id` stored in metadata JSONB but not SQL-filtered today; `created_at` NOT in vector metadata |
| **Vector** (`MemoryVectorStorage`) | In-memory HashMap | **App-level post-retrieval** via same `matches_tenant_filter` method | Same metadata structure as PgVector |
| **Graph** (`MemoryGraphStorage`) | In-memory adjacency lists | **App-level** in `search_nodes()`, `get_popular_nodes_with_degree()` | `tenant_id`, `workspace_id` in node/edge properties; no `document_id` on graph nodes |
| **KV** (`PgKvStorage`/`MemoryKvStorage`) | PostgreSQL / HashMap | Key-prefix pattern (`{doc_id}-metadata`) + tenant check in handler code | Full document metadata available (`created_at`, `title`, etc.) |
| **RLS** (`RlsContext`) | PostgreSQL session vars | **DB-level**: `set_tenant_context()` sets session vars for RLS policies | ❌ Not used for vector queries; exists for graph/KV PostgreSQL tables only |
| **Workspace Vector** (`PgWorkspaceVectorRegistry`) | Per-workspace PostgreSQL tables (`eq_{ns}_ws_{id}_vectors`) | **Structural**: each workspace gets its own table with its own index | Inherits isolation from separate table; no cross-workspace query possible |

#### Vector Metadata Fields Available for Filtering

Examination of all ingestion code paths reveals the exact metadata stored on each vector type:

| Vector Type | Metadata Fields | `document_id` | `created_at` | `tenant_id` | `workspace_id` |
|-------------|----------------|:---:|:---:|:---:|:---:|
| **Chunk** (`text_insert.rs`, `file_upload.rs`, `text_upload.rs`, `ingestion.rs`) | `type`, `document_id`, `index`, `content`, `start_line`, `end_line`, `start_offset`, `end_offset`, `token_count`, `tenant_id`, `workspace_id` | ✅ | ❌ | ✅ | ✅ |
| **Entity** (`merger/entity.rs`, `text_insert.rs`, `file_upload.rs`, `text_upload.rs`) | `type`, `entity_name`, `entity_type`, `description`, `document_id`, `source_chunk_ids`, `source_document_id`, `source_file_path`, `tenant_id`, `workspace_id` | ✅ | ❌ | ✅ | ✅ |
| **Relationship** (`merger/relationship.rs`) | `type`, `src_id`, `tgt_id`, `keywords`, `relation_type`, `description`, `source_chunk_id`, `source_document_id`, `source_file_path`, `tenant_id`, `workspace_id` | ✅ (as `source_document_id`) | ❌ | ✅ | ✅ |

**Critical observation**: `document_id` is present on ALL vector records, but `created_at` is NOT. Date filtering requires a KV metadata lookup to resolve document IDs first.

#### Where `matches_tenant_filter` Operates

```
Vector Search (pgvector SQL: ORDER BY embedding <=> $1 LIMIT $k)
   │
   ▼ Returns top-k by cosine similarity regardless of tenant
   │
   ├── .filter(|r| r.score >= self.config.min_score)        ← score threshold
   ├── .filter(|r| matches_tenant_filter(&r.metadata, ...))  ← tenant post-filter (17 sites)
   └── .take(self.config.max_chunks)                         ← limit
```

This post-retrieval pattern means PostgreSQL computes cosine similarity for ALL vectors in the table (tenant-agnostic), then the app discards non-matching results. With the HNSW index, pgvector uses approximate nearest neighbor, so only `k × ef_search` candidates are evaluated — but if 80% of vectors belong to other tenants, 80% of compute is wasted.

#### SQL-Level JSONB Filtering Feasibility

PostgreSQL pgvector supports combining vector search with WHERE clauses:

```sql
-- Current query (no metadata filter)
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_default_vectors
ORDER BY embedding <=> $1::vector
LIMIT $2;

-- With document_id pre-filter (POSSIBLE but not implemented)
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_default_vectors
WHERE metadata->>'document_id' = ANY($2::text[])
ORDER BY embedding <=> $1::vector
LIMIT $3;
```

**Performance implications of JSONB + vector search**:

| Approach | pgvector Index Used? | Requires Extra Index? | Notes |
|----------|:---:|:---:|-------|
| No WHERE clause | ✅ HNSW scan | No | Current behavior — fast ANN |
| `WHERE id = ANY($2)` | ✅ B-tree + HNSW pre-filter | No (PK index) | Already supported via `filter_ids` |
| `WHERE metadata->>'document_id' = ANY($2)` | ⚠️ Depends on pgvector version | GIN index on `metadata` recommended | Pre-filter scan, then ANN on filtered subset |
| `WHERE metadata->>'tenant_id' = $2` | ⚠️ Same as above | GIN index recommended | Could replace 17 `matches_tenant_filter` call sites |

With pgvector ≥ 0.7.0 and `HNSW` index, PostgreSQL can use **iterative index scan** with WHERE predicates. This means adding `WHERE metadata->>'document_id' = ANY(...)` tells the HNSW index to skip non-matching vectors during traversal rather than computing cosine similarity for them.

However, this requires:
1. A GIN index on the `metadata` column: `CREATE INDEX ON eq_*_vectors USING GIN (metadata jsonb_path_ops);`
2. pgvector ≥ 0.7.0 for efficient pre-filtered ANN search
3. Changes to the `VectorStorage` trait (breaking change to add metadata filter parameter)
4. Changes to both `PgVectorStorage` and `MemoryVectorStorage` implementations

---

### Filtering Strategy: Three-Tier Approach

Given the architectural constraints, we adopt a **three-tier strategy** that balances implementation complexity, performance, and future scalability:

```
                          Filtering Tiers
┌────────────────────────────────────────────────────────────┐
│ TIER 1 (MVP — this implementation)                         │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ API PRE-RESOLUTION + POST-RETRIEVAL CONTEXT FILTER     │ │
│ │                                                        │ │
│ │ 1. API handler scans KV metadata for matching doc IDs  │ │
│ │ 2. Passes allowed_document_ids to engine request       │ │
│ │ 3. Engine runs normal vector search                    │ │
│ │ 4. Context filtered by document_id AFTER retrieval     │ │
│ │    (same level as matches_tenant_filter)               │ │
│ │                                                        │ │
│ │ Cost: ~1ms KV scan + ~0.1ms per filtered result       │ │
│ │ Blast radius: 3 query entry points                     │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ TIER 2 (Future — SQL pre-filter)                           │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ VECTOR STORAGE METADATA FILTER                          │ │
│ │                                                        │ │
│ │ 1. Add metadata_filter param to VectorStorage::query() │ │
│ │ 2. PgVector: WHERE metadata->>'document_id' = ANY($x) │ │
│ │ 3. Memory: HashMap pre-filter                          │ │
│ │ 4. Add GIN index on metadata column                    │ │
│ │                                                        │ │
│ │ Cost: Single SQL query with pre-filtered ANN           │ │
│ │ Blast radius: Trait change + 2 backends + 17 sites     │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ TIER 3 (Scale — dedicated column)                          │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ MATERIALIZED FILTER COLUMNS + PARTIAL INDEXES          │ │
│ │                                                        │ │
│ │ 1. Add document_id, tenant_id TEXT columns to table    │ │
│ │ 2. Add partial HNSW index per tenant/document          │ │
│ │ 3. CREATE INDEX ... WHERE document_id = 'xxx'          │ │
│ │                                                        │ │
│ │ Cost: Index maintenance, migration                     │ │
│ │ Blast radius: Schema migration + all backends          │ │
│ └────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

### Tier 1 Implementation (This PR)

The approach is consistent with how `matches_tenant_filter` already works, and requires no changes to the `VectorStorage` trait or any storage backend.

```
┌─────────────────────────────────────────────────────────────┐
│  1. API Handler (query_execute.rs)                          │
│                                                             │
│  DocumentFilter { date_from, date_to, document_pattern }    │
│           │                                                 │
│           ▼                                                 │
│  Scan KV metadata (same scan already used by list_documents)│
│  → Match created_at against date range                      │
│  → Match title against document_pattern                     │
│  → Collect matching document IDs: Vec<String>               │
│           │                                                 │
│           ▼                                                 │
│  engine_request.allowed_document_ids = Some(matching_ids)   │
└────────────┬────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│  2. Engine Query Pipeline (query_basic.rs)                  │
│                                                             │
│  Normal pipeline: keyword extraction → embedding → vector   │
│  search → mode-specific retrieval                           │
│           │                                                 │
│           ▼ Returns QueryContext                             │
│                                                             │
│  3. Post-Retrieval Filter (ONE centralized function)        │
│                                                             │
│  filter_context_by_document_ids(context, allowed_ids)       │
│  → chunks: keep if chunk.document_id ∈ allowed_ids          │
│  → entities: keep if entity.source_document_id ∈ allowed_ids│
│  → relationships: keep if rel.source_document_id ∈ allowed  │
│           │                                                 │
│           ▼                                                 │
│  4. Truncation → Reranking → LLM Generation                │
└─────────────────────────────────────────────────────────────┘
```

#### Why Tier 1 is Correct for MVP

| Concern | Analysis |
|---------|----------|
| **Consistency** | `matches_tenant_filter` already operates at this same point — adding document filtering here follows the established pattern |
| **Blast radius** | Changes to 3 query entry points (`query`, `query_with_workspace_config`, `query_with_full_config`) vs 17 filter call sites + 1 trait + 2 backends for SQL-level |
| **Performance** | KV metadata scan: ~1ms for 100 docs, ~5ms for 1000 docs. Post-filter: O(n) where n = vector results (typically 10-60 items). Total overhead ≪ NFR-001 10ms budget |
| **Wasted compute** | Vector search may return some results that get filtered out. With HNSW and `LIMIT k`, the overhead is the cosine distance computation for filtered-out items — negligible vs LLM generation (100-5000ms) |
| **`created_at` not in vectors** | Date filtering requires KV lookup regardless — pushing to SQL level would need a JOIN or sub-select, losing HNSW index benefit |
| **No trait changes** | `VectorStorage` trait stays unchanged; memory backend, pgvector backend, workspace vector registry all untouched |

#### Why NOT to Do Tier 2/3 Now

1. **`created_at` is not in vector metadata** — SQL-level date filtering would require either:
   - Adding `created_at` to all vector upsert calls (ingestion change + backfill migration)
   - JOIN with a documents table (loses HNSW index efficiency)
   
2. **Trait change is breaking** — `VectorStorage::query()` signature change affects:
   - `PgVectorStorage` (production)
   - `MemoryVectorStorage` (tests)
   - `PgWorkspaceVectorRegistry` (workspace isolation)
   - All 23+ callers of `vector_storage.query()`

3. **Premature optimization** — the typical filter selectivity (10-50% of documents) means post-retrieval filtering discards a manageable number of results. The LLM call dominates latency by 100x.

#### Migration Path to Tier 2

When Tier 2 is needed (>10K documents, highly selective filters):

1. Add `created_at`, `document_id` columns to vector table schema
2. Add GIN index on metadata or B-tree index on new columns
3. Add `metadata_filter: Option<HashMap<String, Vec<String>>>` to `VectorStorage::query()`
4. `PgVectorStorage`: add `WHERE metadata->>'document_id' = ANY($x)` to SQL
5. `MemoryVectorStorage`: add pre-filter on metadata HashMap
6. The `allowed_document_ids` field on `QueryRequest` stays — the API resolution logic is the same

### Backend Changes

#### 1. New DTO: `DocumentFilter` (shared between query + listing)

```rust
// In edgequake-api/src/handlers/query_types.rs
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
pub struct DocumentFilter {
    /// Start date (inclusive). ISO 8601.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,

    /// End date (inclusive). ISO 8601.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,

    /// Document name pattern (case-insensitive substring match).
    /// Comma-separated for OR logic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_pattern: Option<String>,
}
```

#### 2. Add to `QueryRequest` (API + Engine)

```rust
// API QueryRequest (query_types.rs)
pub struct QueryRequest {
    // ... existing fields ...

    /// Optional document filter to narrow retrieval scope.
    /// @implements SPEC-005: Date & pattern filters
    #[serde(default)]
    pub document_filter: Option<DocumentFilter>,
}

// Engine QueryRequest (edgequake-query/src/engine.rs)
pub struct QueryRequest {
    // ... existing fields ...

    /// Pre-resolved document IDs that match the user's date/pattern filters.
    /// When set, only chunks/entities/relationships from these documents are
    /// included in the query context. Resolved by the API layer from
    /// DocumentFilter criteria via KV metadata scan.
    /// @implements SPEC-005: Document date and pattern filters
    pub allowed_document_ids: Option<Vec<String>>,
}
```

#### 3. Add to `ListDocumentsRequest` (document listing)

```rust
// In documents_types/listing.rs
pub struct ListDocumentsRequest {
    pub page: usize,
    pub page_size: usize,

    /// Date filter: start date (inclusive, ISO 8601).
    #[serde(default)]
    pub date_from: Option<String>,

    /// Date filter: end date (inclusive, ISO 8601).
    #[serde(default)]
    pub date_to: Option<String>,

    /// Document name pattern filter.
    #[serde(default)]
    pub document_pattern: Option<String>,
}
```

#### 4. API-Level Document ID Resolution

The API handler (`query_execute.rs`) resolves `DocumentFilter` → `Vec<String>` of matching document IDs by scanning KV metadata. This is the same scan pattern already used by `list_documents`.

```rust
/// Resolve DocumentFilter to matching document IDs via KV metadata scan.
/// Returns None if no filter is active (all documents match).
async fn resolve_document_filter(
    kv_storage: &dyn KVStorage,
    filter: &DocumentFilter,
    tenant_id: &Option<String>,
    workspace_id: &Option<String>,
) -> Result<Option<Vec<String>>> {
    if filter.date_from.is_none() && filter.date_to.is_none()
        && filter.document_pattern.is_none() {
        return Ok(None); // No filter active
    }

    let keys = kv_storage.keys().await?;
    let metadata_keys: Vec<String> = keys.iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    let metadata_values = kv_storage.get_by_ids(&metadata_keys).await?;
    let mut matching_ids = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
            // Check tenant/workspace match first (security boundary)
            // ... tenant/workspace check ...

            // Check date range
            if let Some(ref from) = filter.date_from {
                if let Some(created) = obj.get("created_at").and_then(|v| v.as_str()) {
                    if created < from.as_str() { continue; }
                } else { continue; } // No created_at → excluded by date filter
            }
            if let Some(ref to) = filter.date_to {
                if let Some(created) = obj.get("created_at").and_then(|v| v.as_str()) {
                    if created > to.as_str() { continue; }
                } else { continue; }
            }

            // Check document pattern
            if let Some(ref pattern) = filter.document_pattern {
                let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let combined = title.to_lowercase();
                let any_match = pattern.split(',')
                    .any(|p| combined.contains(&p.trim().to_lowercase()));
                if !any_match { continue; }
            }

            matching_ids.push(id.to_string());
        }
    }

    Ok(Some(matching_ids))
}
```

#### 5. Post-Retrieval Context Filter (Engine Level)

A single centralized function applied in the 3 query entry points, after mode-specific retrieval returns a `QueryContext` and before truncation/reranking:

```rust
/// Filter a QueryContext to only include items from allowed documents.
/// Applied after vector search, before truncation and LLM generation.
fn filter_context_by_document_ids(
    context: &mut QueryContext,
    allowed_ids: &[String],
) {
    let id_set: HashSet<&str> = allowed_ids.iter().map(|s| s.as_str()).collect();

    context.chunks.retain(|chunk| {
        chunk.document_id.as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(false) // No document_id → excluded
    });

    context.entities.retain(|entity| {
        entity.source_document_id.as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(true) // Entities may span multiple documents
    });

    context.relationships.retain(|rel| {
        rel.source_document_id.as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(true) // Relationships may span documents
    });
}
```

This function is called in:
- `query()` (query_basic.rs) — default vector storage
- `query_with_workspace_config()` (query_workspace.rs) — workspace embedding + vector
- `query_with_full_config()` (query_workspace.rs) — workspace + LLM override

### Frontend Changes

#### 1. Query Interface: Filter Panel

Add a collapsible "Filters" section above or beside the query input:

```
┌──────────────────────────────────────────────────┐
│ 🔍 Query: [                                   ] │
│                                                  │
│ ▼ Filters                                        │
│ ┌──────────────────────────────────────────────┐ │
│ │ Date Range: [From: ____] → [To: ____]       │ │
│ │ Quick: [7d] [30d] [90d] [This Year] [Clear] │ │
│ │                                              │ │
│ │ Documents: [Filter by name... ___________]   │ │
│ └──────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

#### 2. Document Manager: Date Range Filter

Add date range inputs to the existing toolbar section:

```
┌──────────────────────────────────────────────────┐
│ [Search...] [Status ▼] [Sort ▼] [Date Range 📅] │
│                                                  │
│ ┌── Date Filter ──────────────────────────────┐  │
│ │ From: [____] To: [____]                     │  │
│ │ [7d] [30d] [90d] [This Year] [Clear]        │  │
│ └─────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

#### 3. API Client Updates

```typescript
// In edgequake.ts
export interface DocumentFilter {
  date_from?: string;  // ISO 8601
  date_to?: string;    // ISO 8601
  document_pattern?: string;
}

export interface QueryRequest {
  // ... existing fields ...
  document_filter?: DocumentFilter;
}

export async function getDocuments(
  params?: PaginationParams & {
    status?: string;
    date_from?: string;
    date_to?: string;
    document_pattern?: string;
  }
): Promise<DocumentsListResult> { ... }
```

---

## Implementation Plan

### Phase 1: Backend (Core)

| Step | Component | File | Change |
|------|-----------|------|--------|
| 1.1 | DTO | `query_types.rs` | Add `DocumentFilter` struct ✅ |
| 1.2 | DTO | `query_types.rs` | Add `document_filter` to API `QueryRequest` ✅ |
| 1.3 | DTO | `listing.rs` | Add date/pattern params to `ListDocumentsRequest` ✅ |
| 1.4 | Engine | `engine.rs` | Add `allowed_document_ids: Option<Vec<String>>` to engine `QueryRequest` ✅ |
| 1.5 | Resolver | `query_execute.rs` | Add `resolve_document_filter()` function: scans KV metadata, resolves `DocumentFilter` → `Vec<String>` of matching document IDs |
| 1.6 | Resolver | `query_execute.rs` | Wire resolver: call `resolve_document_filter()`, set `engine_request.allowed_document_ids` |
| 1.7 | Filter | `edgequake-query/src/context_filter.rs` | New module: `filter_context_by_document_ids(context, allowed_ids)` — centralized post-retrieval filter |
| 1.8 | Engine | `query_basic.rs` | Call `filter_context_by_document_ids` after mode returns `QueryContext`, before truncation |
| 1.9 | Engine | `query_workspace.rs` | Same for `query_with_workspace_config` and `query_with_full_config` |
| 1.10 | Handler | `list.rs` | Add date/pattern filtering to document listing (same scan logic as resolver) |
| 1.11 | Tests | Unit tests | `DocumentFilter` serialization, `resolve_document_filter` with mock KV, `filter_context_by_document_ids` with various document ID sets |

### Phase 2: Frontend

| Step | Component | File | Change |
|------|-----------|------|--------|
| 2.1 | Types | `types/index.ts` | Add `DocumentFilter` TypeScript type |
| 2.2 | API | `lib/api/edgequake.ts` | Update `query()` and `getDocuments()` |
| 2.3 | Hook | `hooks/use-query-filters.ts` | New hook for query filter state |
| 2.4 | UI | `components/query/query-filters.tsx` | Date range + pattern filter panel |
| 2.5 | UI | `components/query/query-interface.tsx` | Integrate filter panel |
| 2.6 | Hook | `hooks/use-document-filtering.ts` | Add date range filtering |
| 2.7 | UI | `components/documents/document-date-filter.tsx` | Date filter for doc list |
| 2.8 | UI | `components/documents/document-toolbar-section.tsx` | Integrate date filter |

### Phase 3: Testing & Documentation

| Step | Description |
|------|-------------|
| 3.1 | E2E: Verify date filter on query page |
| 3.2 | E2E: Verify date filter on documents page |
| 3.3 | E2E: Verify pattern filter on query page |
| 3.4 | Unit tests for date parsing and comparison |
| 3.5 | Update CHANGELOG.md |
| 3.6 | Update API documentation |

---

## API Contract

### POST `/api/v1/query`

```json
{
  "query": "What are the key financial metrics?",
  "mode": "hybrid",
  "document_filter": {
    "date_from": "2025-01-01T00:00:00Z",
    "date_to": "2025-03-31T23:59:59Z",
    "document_pattern": "financial,quarterly"
  }
}
```

### GET `/api/v1/documents`

```
GET /api/v1/documents?page=1&page_size=20&date_from=2025-01-01&date_to=2025-03-31&document_pattern=report
```

---

## Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| Both dates null | No date filtering (all documents) |
| `date_from` > `date_to` | Return 400 Bad Request |
| Invalid date format | Return 400 Bad Request |
| Pattern with no matches | Empty results (0 chunks in context) |
| Pattern with special chars | Escape and treat as literal |
| Document has no `created_at` | Excluded if date filter is active (conservative: dates unknown) |
| Existing queries without filters | Backward compatible (no change) |
| `allowed_document_ids` is `Some(vec![])` (filter active but 0 matches) | Return empty context → LLM replies "no matching documents found" |
| `allowed_document_ids` is `None` (no filter active) | No-op — all documents included (default behavior) |
| Chunk has no `document_id` in metadata | Excluded when filter is active (orphan chunk) |
| Entity spans multiple documents | Kept if ANY `source_document_id` is in allowed set |
| Relationship spans multiple documents | Kept if `source_document_id` is in allowed set |
| KV metadata scan returns >10K documents | Performance: ~50ms for 10K documents — still within budget for query frequency. Consider caching for Tier 2. |

---

## Backward Compatibility

- All new fields are optional (`Option<T>` in Rust, `?` in TypeScript)
- Default behavior (no filters) is identical to current behavior
- Existing API clients require no changes
- No database migration needed (metadata already has `created_at` in KV, `document_id` in vectors)
- No `VectorStorage` trait changes in Tier 1 — trait signature unchanged
- No storage backend changes in Tier 1 — post-retrieval filter only

---

## Key Architectural Constraints

| Constraint | Impact | Mitigation |
|------------|--------|------------|
| `created_at` NOT in vector metadata | Cannot date-filter at SQL level in vector queries | Resolve doc IDs via KV scan first, then filter context by `document_id` |
| `document_id` IS in vector metadata | Could be SQL-filtered but requires trait change | Tier 1: post-retrieval; Tier 2: add JSONB WHERE clause |
| `matches_tenant_filter` is post-retrieval (17 sites) | Establishes precedent — post-filter is the norm | Follow same pattern for document filtering |
| Workspace isolation via separate tables (SPEC-033) | No cross-workspace vector query possible | Document filtering is intra-workspace only |
| HNSW index with WHERE clause (pgvector ≥ 0.7.0) | Iterative scan could skip non-matching vectors | Tier 2 optimization target |

---

## Success Criteria

- [x] Date filter narrows RAG query to matching documents only
- [x] Document pattern filter restricts query to matching documents
- [x] Document list supports server-side date/pattern filtering
- [x] UI provides intuitive date range picker and pattern input
- [x] All filters are optional and backward compatible
- [x] E2E test demonstrates working filters
- [x] CHANGELOG updated with feature entry
