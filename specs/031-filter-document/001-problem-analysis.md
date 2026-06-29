# SPEC-031 / 001 — Problem Analysis: Current State & Gap Identification

> **Lens**: Engineering (backend + frontend deep-dive)  
> **Cross-refs**: SPEC-005, `document_filter_resolver.rs`, `context_filter.rs`, `QueryDocumentFilter`

---

## 1. Current Architecture Map

### 1.1 Backend Data Flow (Query with Document Filter)

```
Client POST /api/v1/query
  {
    "query": "...",
    "document_filter": {
      "date_from": "2025-01-01T00:00:00Z",    // optional
      "date_to":   "2025-12-31T23:59:59Z",    // optional
      "document_pattern": "report,invoice"     // optional
    }
  }
           |
           v
  query_execute.rs / query_stream.rs
           |
           | if document_filter is Some:
           v
  document_filter_resolver.rs
    resolve_document_filter()
    - Calls load_scoped_document_metadata_entries(kv_storage, tenant_ctx)
    - Scans ALL {doc_id}-metadata KV keys for the workspace
    - Filters by:
        created_at >= date_from (if set)
        created_at <= date_to   (if set)
        title.contains(pattern) (OR across comma-separated patterns)
    - Returns Vec<String> of matching document IDs
           |
           v
  edgequake_query::QueryRequest
    .allowed_document_ids = Some(matched_ids)
           |
           v
  QueryEngine.execute(request)
    -> query_pipeline.rs line ~140:
       context_filter::filter_context_by_document_ids(
           &mut context,
           request.allowed_document_ids.as_deref()
       )
    -> Filters retrieved chunks/entities post-retrieval
           |
           v
  QueryResponse { answer, context, stats }
```

### 1.2 Frontend Data Flow (Query with Document Filter)

```
QueryInterface (query-interface.tsx)
  |
  +-- QuerySettingsSheet (query-settings-sheet.tsx)
        |
        +-- QueryDocumentFilter (query-document-filter.tsx)
              Popover with:
              - date_from  (input[type=date])
              - date_to    (input[type=date])
              - document_pattern (text input)
              |
              v
              documentFilter: DocumentFilter | undefined
              |
              v (via onDocumentFilterChange -> setQuerySettings)

useQueryInterface hook
  querySettings.documentFilter -> DocumentFilter | undefined
  |
  v  (on submit)
sendQuery(chat.ts):
  body.document_filter = querySettings.documentFilter
```

---

## 2. Identified Gaps

### GAP-001: No Explicit Document ID Selection

**Problem**: Users can only filter by fuzzy name pattern or date range. There is no way to say "query ONLY these 3 specific documents: `doc-abc`, `doc-def`, `doc-ghi`".

**Why it matters**: Pattern matching is non-deterministic. A pattern like "report" matches ALL documents with "report" in the title. When a user wants to compare two specific quarterly reports, they need exact selection.

**Current workaround**: None. Users must craft very specific patterns and hope for no false positives.

**Root cause**: `DocumentFilter` has no `document_ids` field. The `allowed_document_ids` in the engine accepts `Vec<String>` but can only be populated via the pattern resolver, not directly.

```rust
// edgequake-query/src/types.rs — current struct
pub struct QueryRequest {
    // ...
    pub allowed_document_ids: Option<Vec<String>>,  // EXISTS but...
    // no direct path from client to this field without going through resolver
}

// edgequake-api/src/handlers/query_types.rs — DocumentFilter DTO
pub struct DocumentFilter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub document_pattern: Option<String>,  // <-- only way in today
    // MISSING: document_ids: Option<Vec<String>>
}
```

### GAP-002: No Type-Ahead Document Search Endpoint

**Problem**: There is no lightweight API for searching documents by partial title for picker UI needs.

**Current state**: `GET /api/v1/documents?document_pattern=&page_size=&page=` exists but returns full `DocumentSummary` objects with all metadata. For a type-ahead picker, this is over-engineered:
- Returns `entity_count`, `cost_usd`, `chunk_count`, etc.
- No `status` pre-filter to exclude failed/processing documents
- Not optimized for < 200ms latency at scale

**Missing**: `GET /api/v1/documents/search?q=&status=completed&page_size=20` returning `{ id, title, status }[]` only.

### GAP-003: Document Scope Is Invisible in Query UI

**Problem**: The document filter lives inside the settings sheet (3 clicks to reach: Settings → scroll → filter popover). Users don't realize their query is scoped unless they open the sheet.

**Current UX path**:
```
[Query] button  → Settings gear icon → Scroll to "Document Filter" → Open popover
```

**Desired UX**:
```
Scope indicator always visible in query bar with pills
```

### GAP-004: Scope Not Reflected in Query Bar Context

**Problem**: Even when a document filter is active, the query bar shows no indication. A user can type a query not realizing it will only search 3 documents.

**Impact**: Silent wrong results — user gets answer scoped to 3 docs and doesn't know why it's incomplete.

### GAP-005: No Explicit "All Workspace" Affordance

**Problem**: The absence of any filter = all workspace, but there's no explicit signal of this. Adding pills changes the scope; removing the last pill should restore "All workspace" explicitly.

---

## 3. What Already Works Well (SPEC-005 Wins)

| Feature                               | Status        | Notes                                                     |
| ------------------------------------- | ------------- | --------------------------------------------------------- |
| Date range filtering                  | ✅ Implemented | `date_from` + `date_to` in `DocumentFilter`               |
| Name pattern filtering                | ✅ Implemented | Comma-separated OR, case-insensitive                      |
| Engine-level chunk filtering          | ✅ Implemented | `context_filter::filter_context_by_document_ids`          |
| `allowed_document_ids` in engine      | ✅ Implemented | Accepts `Vec<String>` — just needs direct population path |
| Filter badge count in filter button   | ✅ Implemented | Badge shows `activeFilterCount`                           |
| Document list API with pattern filter | ✅ Implemented | `GET /documents?document_pattern=`                        |

---

## 4. Dependency Analysis

### What SPEC-031 Builds On

```
SPEC-005 (filter foundation)
    |
    +-- DocumentFilter struct (extend with document_ids[])
    +-- document_filter_resolver.rs (add ID-first path)
    +-- GET /documents?document_pattern= (reuse as picker search base)

edgequake-query/context_filter.rs
    |
    +-- filter_context_by_document_ids() ALREADY CORRECT
    +-- No changes needed — just needs IDs fed correctly

useQuerySettings hook (frontend)
    |
    +-- Extend to persist document scope (localStorage key)
```

### What Must Not Change (Backward Compatibility)

- `DocumentFilter` serialization: adding `document_ids` is additive (nullable field)
- `GET /documents` listing API signature: unchanged
- `query_execute.rs` and `query_stream.rs` resolver call path: extend, don't replace
- All existing SPEC-005 filter UI in `QueryDocumentFilter`: retained as-is in settings sheet

---

## 5. Volume & Performance Baseline

To correctly spec the search endpoint, understanding scale:

| Metric               | Current observed  | Planning ceiling                     |
| -------------------- | ----------------- | ------------------------------------ |
| Docs per workspace   | 10–500 typical    | 10,000 max                           |
| KV scan for metadata | < 5ms at 100 docs | < 50ms at 1,000 docs                 |
| p99 KV scan          | ~10ms             | ~80ms (needs optimization at 5,000+) |

For a workspace with 10,000 documents, a full KV scan for type-ahead is acceptable only if:
1. Results are returned as minimal projections (no chunk counts, no embeddings)
2. A server-side `page_size=20` cap is enforced
3. The query is debounced 300ms client-side

At 10,000 documents a full scan may approach 100ms; if this becomes a concern, a future spec should index document titles in a dedicated search structure (out of scope here).

---

## 6. Contract Summary for Adjacent Systems

| System                            | Change Type                        | Impact                      |
| --------------------------------- | ---------------------------------- | --------------------------- |
| `DocumentFilter` DTO              | Additive new field `document_ids?` | Non-breaking (null default) |
| `document_filter_resolver.rs`     | Extended: IDs bypass KV scan       | No breaking change          |
| Query stream + execute handlers   | Minor: union IDs from resolver     | No breaking change          |
| `GET /documents` route            | No change                          | No impact                   |
| New `GET /documents/search` route | New route                          | Additive                    |
| `QueryDocumentFilter` component   | Unchanged (kept in settings)       | No impact                   |
| New `QueryScopeBar` component     | New component in query header      | Additive                    |
| `useQuerySettings` hook           | Extended: `documentIds?: string[]` | Additive                    |
| MCP `query` tool                  | Extended: `document_ids` param     | Additive                    |
