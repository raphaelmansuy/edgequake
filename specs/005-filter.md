# SPEC-005: Date & Document Pattern Filters for Queries and Document Listing

> **Issue**: [#75 — Add optional date Filter parameters to a user request](https://github.com/raphaelmansuy/edgequake/issues/75)
> **Status**: Accepted
> **Priority**: High
> **Complexity**: Medium

## Summary

Add optional **date range** and **document pattern** filter parameters to both **RAG queries** and **document listing**. These filters are applied *before* retrieval (pre-filter), narrowing the search space to only documents matching the criteria.

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

### Filter Application Points

The filters are applied at two levels:

```
┌─────────────────────────────────────────────────┐
│  User Request (API Layer)                       │
│  { query, date_filter, document_filter }        │
└──────────────────┬──────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────┐
│  Pre-Retrieval Filter (Engine Level)            │
│  → Filter vector search results by metadata     │
│  → Check created_at against date range          │
│  → Check document_id → title against pattern    │
│  → Applied BEFORE context building              │
└──────────────────┬──────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────┐
│  Response (only filtered documents in context)  │
└─────────────────────────────────────────────────┘
```

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

    /// Date filter: start date (inclusive, ISO 8601).
    pub date_from: Option<String>,

    /// Date filter: end date (inclusive, ISO 8601).
    pub date_to: Option<String>,

    /// Document name pattern filter.
    pub document_pattern: Option<String>,
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

#### 4. Extend `matches_tenant_filter` in SOTA Engine

The existing `matches_tenant_filter` method is extended with date and pattern checks. This method is called on every vector search result, making it the ideal pre-filter point.

```rust
// Pseudocode for filter extension
fn matches_document_filter(
    &self,
    metadata: &serde_json::Value,
    date_from: Option<&str>,
    date_to: Option<&str>,
    document_pattern: Option<&str>,
) -> bool {
    // Date check on created_at
    if let Some(from) = date_from {
        if let Some(created) = metadata.get("created_at").and_then(|v| v.as_str()) {
            if created < from { return false; }
        }
    }
    if let Some(to) = date_to {
        if let Some(created) = metadata.get("created_at").and_then(|v| v.as_str()) {
            if created > to { return false; }
        }
    }

    // Pattern check on title/file_name
    if let Some(pattern) = document_pattern {
        let title = metadata.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let file = metadata.get("file_name").and_then(|v| v.as_str()).unwrap_or("");
        let combined = format!("{} {}", title, file).to_lowercase();
        let matches = pattern.split(',')
            .any(|p| combined.contains(&p.trim().to_lowercase()));
        if !matches { return false; }
    }

    true
}
```

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
| 1.1 | DTO | `query_types.rs` | Add `DocumentFilter` struct |
| 1.2 | DTO | `query_types.rs` | Add `document_filter` to `QueryRequest` |
| 1.3 | DTO | `listing.rs` | Add date/pattern params to `ListDocumentsRequest` |
| 1.4 | Engine | `engine.rs` | Add date/pattern fields to engine `QueryRequest` |
| 1.5 | Handler | `query_execute.rs` | Pass filter from API to engine request |
| 1.6 | Engine | `sota_engine/*.rs` | Add `matches_document_filter` method |
| 1.7 | Handler | `list.rs` | Add date/pattern filtering to document list |
| 1.8 | Tests | `query_types.rs` | Unit tests for `DocumentFilter` serialization |

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
| Document has no `created_at` | Excluded if date filter is active |
| Existing queries without filters | Backward compatible (no change) |

---

## Backward Compatibility

- All new fields are optional (`Option<T>` in Rust, `?` in TypeScript)
- Default behavior (no filters) is identical to current behavior
- Existing API clients require no changes
- No database migration needed (metadata already has `created_at`)

---

## Success Criteria

- [ ] Date filter narrows RAG query to matching documents only
- [ ] Document pattern filter restricts query to matching documents
- [ ] Document list supports server-side date/pattern filtering
- [ ] UI provides intuitive date range picker with presets
- [ ] All filters are optional and backward compatible
- [ ] E2E test demonstrates working filters
- [ ] CHANGELOG updated with feature entry
