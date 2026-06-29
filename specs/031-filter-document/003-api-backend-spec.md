# SPEC-031 / 003 — API & Backend Specification

> **Lens**: System Engineer · AI/RAG Engineer  
> **Cross-refs**: SPEC-005, `document_filter_resolver.rs`, `context_filter.rs`, `query_types.rs`

---

## 1. Overview

Three changes are needed in the backend:

1. **Extend `DocumentFilter` DTO** with `document_ids: Option<Vec<String>>`
2. **Extend `document_filter_resolver.rs`** to short-circuit KV scan when IDs are explicit
3. **Add `GET /api/v1/documents/search`** — lightweight type-ahead endpoint

---

## 2. `DocumentFilter` DTO Extension

### 2.1 Current Struct (`edgequake-api/src/handlers/query_types.rs`)

```rust
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DocumentFilter {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub document_pattern: Option<String>,
}
```

### 2.2 Proposed Struct

```rust
/// Optional filter to restrict RAG context to a subset of documents.
///
/// Fields are AND-combined. Within `document_ids` and `document_pattern`,
/// matches are OR-unioned (any match includes the document).
///
/// @implements SPEC-031: Explicit document scope selection
/// @implements SPEC-005: Date and pattern filters (retained, unchanged)
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
pub struct DocumentFilter {
    // ── SPEC-005 fields (unchanged) ─────────────────────────────────────────

    /// Start date (inclusive), ISO 8601. Documents created on or after this date.
    /// @implements SPEC-005
    #[serde(default)]
    pub date_from: Option<String>,

    /// End date (inclusive), ISO 8601. Documents created on or before this date.
    /// @implements SPEC-005
    #[serde(default)]
    pub date_to: Option<String>,

    /// Case-insensitive title substring. Comma-separated = OR.
    /// @implements SPEC-005
    #[serde(default)]
    pub document_pattern: Option<String>,

    // ── SPEC-031 new field ───────────────────────────────────────────────────

    /// Explicit document IDs to restrict query scope.
    ///
    /// When set, only these documents are used as RAG context, subject also
    /// to any active `date_from`/`date_to`/`document_pattern` constraints
    /// (AND logic across field types, OR logic within each field type).
    ///
    /// An empty list `[]` is treated identically to `null` (no filtering),
    /// ensuring clients that send `[]` do not accidentally produce empty results.
    ///
    /// IDs that do not exist in the current workspace are silently ignored.
    ///
    /// @implements SPEC-031: Explicit document scope selection
    #[serde(default)]
    pub document_ids: Option<Vec<String>>,
}
```

### 2.3 `is_empty()` Helper

```rust
impl DocumentFilter {
    /// Returns true when no filter criteria are set (all-pass).
    pub fn is_empty(&self) -> bool {
        self.date_from.is_none()
            && self.date_to.is_none()
            && self.document_pattern.is_none()
            && self.document_ids.as_ref().map_or(true, |ids| ids.is_empty())
    }
}
```

### 2.4 Backward Compatibility

Existing JSON without `document_ids`:
```json
{"date_from": "2025-01-01T00:00:00Z", "document_pattern": "report"}
```
Deserializes correctly — `document_ids` defaults to `None`. No migration needed.

---

## 3. `document_filter_resolver.rs` Changes

### 3.1 Updated Resolution Logic

```rust
/// Resolve a `DocumentFilter` into a list of matching document IDs.
///
/// Resolution order:
/// 1. If filter is empty → return None (no-op, no scan)
/// 2. If `document_ids` is set (non-empty) → start with those IDs as candidate set
/// 3. If `document_pattern` is set → KV scan, add matching IDs to candidate set
/// 4. Intersect candidate set with date range filter
///
/// "Candidate set" logic:
/// - document_ids present → seed with explicit IDs
/// - document_pattern present → union with KV pattern-matched IDs
/// - If BOTH are set → union of explicit IDs + pattern-matched IDs
///   (either satisfies membership in candidate set)
/// - Then apply date filter as AND filter across the candidate set
///
/// Returns `None` if no filtering required (all documents allowed).
/// Returns `Some(vec![])` if filtering required but nothing matched.
///
/// @implements SPEC-031: Explicit document scope
/// @implements SPEC-005: Date and pattern filters
pub async fn resolve_document_filter(
    kv_storage: &dyn KVStorage,
    filter: &DocumentFilter,
    tenant_id: &Option<String>,
    workspace_id: &Option<String>,
) -> Result<Option<Vec<String>>, ApiError> {
    // Fast path: no filter criteria
    if filter.is_empty() {
        return Ok(None);
    }

    let has_explicit_ids = filter.document_ids.as_ref()
        .map_or(false, |ids| !ids.is_empty());
    let has_pattern = filter.document_pattern.is_some();
    let has_date_filter = filter.date_from.is_some() || filter.date_to.is_some();

    // If ONLY explicit IDs (no date/pattern) → skip KV scan entirely
    if has_explicit_ids && !has_pattern && !has_date_filter {
        let ids = filter.document_ids.as_ref().unwrap().clone();
        return Ok(Some(deduplicate(ids)));
    }

    // Need KV scan for pattern matching or date filtering
    let tenant_ctx = TenantContext {
        tenant_id: tenant_id.clone(),
        workspace_id: workspace_id.clone(),
        user_id: None,
    };
    let metadata_values = load_scoped_document_metadata_entries(kv_storage, &tenant_ctx)
        .await?
        .into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<_>>();

    if metadata_values.is_empty() {
        return Ok(Some(Vec::new()));
    }

    // Parse patterns once
    let patterns = parse_patterns(filter.document_pattern.as_deref());

    // Build candidate set from explicit IDs
    let explicit_id_set: std::collections::HashSet<String> = filter
        .document_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect())
        .unwrap_or_default();

    let mut matched_ids = Vec::new();

    for value in &metadata_values {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };
        let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };

        // Date range filter (AND — applied to all candidates)
        if !passes_date_filter(obj, &filter.date_from, &filter.date_to, doc_id) {
            continue;
        }

        // Membership: explicit ID OR pattern match
        let in_explicit = explicit_id_set.contains(doc_id);
        let in_pattern = if patterns.is_empty() {
            // No pattern → don't use pattern for membership
            false
        } else {
            let title = obj.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            patterns.iter().any(|p| title.contains(p.as_str()))
        };

        // If explicit IDs are set: doc must be in explicit OR pattern
        // If no explicit IDs: doc must match pattern (if set)
        // If neither explicit nor pattern: date-only filter — all pass
        let passes_membership = if has_explicit_ids || has_pattern {
            in_explicit || in_pattern
        } else {
            true // date-only filter: all docs that passed date check are included
        };

        if passes_membership {
            matched_ids.push(doc_id.to_string());
        }
    }

    debug!(
        filter.document_ids = ?filter.document_ids,
        filter.document_pattern = ?filter.document_pattern,
        matched_count = matched_ids.len(),
        "Document filter resolved"
    );

    Ok(Some(matched_ids))
}

fn deduplicate(ids: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    ids.into_iter().filter(|id| seen.insert(id.clone())).collect()
}

fn parse_patterns(pattern: Option<&str>) -> Vec<String> {
    pattern
        .map(|p| p.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect())
        .unwrap_or_default()
}

fn passes_date_filter(
    obj: &serde_json::Map<String, serde_json::Value>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    doc_id: &str,
) -> bool {
    let created_at = obj.get("created_at").and_then(|v| v.as_str());

    if let Some(ref date_from) = date_from {
        match created_at {
            Some(ca) if ca >= date_from.as_str() => {}
            Some(_) => return false,
            None => {
                warn!(document_id = %doc_id, "No created_at — excluded by date_from filter");
                return false;
            }
        }
    }

    if let Some(ref date_to) = date_to {
        match created_at {
            Some(ca) if ca <= date_to.as_str() => {}
            Some(_) => return false,
            None => {
                warn!(document_id = %doc_id, "No created_at — excluded by date_to filter");
                return false;
            }
        }
    }

    true
}
```

### 3.2 Key Behavioral Invariants

| Scenario                                              | Behavior                                         |
| ----------------------------------------------------- | ------------------------------------------------ |
| `document_ids = []`                                   | Treated as `null` → no filtering (all workspace) |
| `document_ids = ["a","b"]`, no date/pattern           | Return `["a","b"]` immediately, no KV scan       |
| `document_ids = ["a"]`, `document_pattern = "report"` | Union: docs matching "report" OR doc "a"         |
| `document_ids = ["a"]`, `date_from = "2025-01-01"`    | KV scan, return doc "a" only if its date passes  |
| `document_ids` contains non-existent ID               | Silently ignored (missing in KV scan)            |
| All criteria set                                      | Date filter is AND; IDs + pattern is OR union    |

---

## 4. New `GET /api/v1/documents/search` Endpoint

### 4.1 Route Registration

```rust
// In routes.rs, before /documents/{document_id}:
.route("/documents/search", get(handlers::search_documents))
```

### 4.2 Request / Response Types

```rust
/// Lightweight document search for type-ahead picker (SPEC-031).
///
/// @implements SPEC-031: Document search endpoint for picker UI
#[derive(Debug, Deserialize, ToSchema)]
pub struct DocumentSearchRequest {
    /// Search query — case-insensitive substring match on document title.
    /// Minimum 1 character. Truncated to 200 characters server-side.
    #[serde(default)]
    pub q: Option<String>,

    /// Maximum number of results (default: 20, max: 50).
    #[serde(default = "default_search_page_size")]
    pub page_size: usize,

    /// Filter by status. Defaults to "completed" to show only usable documents.
    /// Pass "all" to include all statuses.
    #[serde(default = "default_search_status")]
    pub status: Option<String>,
}

fn default_search_page_size() -> usize { 20 }
fn default_search_status() -> Option<String> { Some("completed".to_string()) }

/// Minimal document projection for the scope picker.
///
/// Intentionally minimal — only data needed to display a picker item.
/// Does NOT include chunk counts, cost, embeddings, or entity counts.
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentSearchItem {
    /// Document UUID.
    pub id: String,
    /// Document title (from metadata or file name).
    pub title: String,
    /// Processing status.
    pub status: String,
    /// ISO 8601 creation timestamp.
    pub created_at: Option<String>,
}

/// Response from document search.
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentSearchResponse {
    /// Matching documents (up to page_size).
    pub items: Vec<DocumentSearchItem>,
    /// Total matches found (may exceed `items.len()` if capped).
    pub total: usize,
    /// True when total > items.len().
    pub has_more: bool,
}
```

### 4.3 Handler Implementation

```rust
/// Search documents by title for the scope picker (SPEC-031).
///
/// Lightweight endpoint returning only id/title/status projections.
/// Requires full tenant context (workspace_id + tenant_id).
#[utoipa::path(
    get,
    path = "/api/v1/documents/search",
    tag = "Documents",
    params(
        ("q" = Option<String>, Query, description = "Title search query"),
        ("page_size" = Option<usize>, Query, description = "Max results (default 20, max 50)"),
        ("status" = Option<String>, Query, description = "Status filter (default: completed)"),
    ),
    responses(
        (status = 200, description = "Search results", body = DocumentSearchResponse),
        (status = 400, description = "Invalid request"),
    )
)]
pub async fn search_documents(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<DocumentSearchRequest>,
) -> ApiResult<Json<DocumentSearchResponse>> {
    // Security: require full tenant context
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "search_documents");
        return Ok(Json(DocumentSearchResponse {
            items: vec![],
            total: 0,
            has_more: false,
        }));
    }

    // Cap page_size
    let page_size = params.page_size.min(50);

    // Sanitize query string (prevent abuse)
    let query = params.q
        .as_deref()
        .map(|q| &q[..q.len().min(200)])
        .map(str::to_lowercase);

    // Load metadata (same SSOT as list_documents)
    let metadata_values =
        load_scoped_document_metadata(storage.kv_storage.as_ref(), &tenant_ctx).await?;

    let status_filter = params.status.as_deref()
        .filter(|&s| s != "all");

    let mut items: Vec<DocumentSearchItem> = Vec::new();

    for value in &metadata_values {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        let id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };

        let title = obj.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let status = obj.get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Status filter
        if let Some(required_status) = status_filter {
            if status != required_status {
                continue;
            }
        }

        // Title search filter (substring, case-insensitive)
        if let Some(ref q) = query {
            if !q.is_empty() && !title.to_lowercase().contains(q.as_str()) {
                continue;
            }
        }

        let created_at = obj.get("created_at")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        items.push(DocumentSearchItem {
            id: id.to_string(),
            title,
            status,
            created_at,
        });
    }

    // Sort by created_at descending (most recent first)
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = items.len();
    let has_more = total > page_size;
    items.truncate(page_size);

    Ok(Json(DocumentSearchResponse { items, total, has_more }))
}
```

### 4.4 API Contract

```
GET /api/v1/documents/search?q=report&page_size=10&status=completed

Response 200:
{
  "items": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Q1 2025 Financial Report",
      "status": "completed",
      "created_at": "2025-01-15T10:00:00Z"
    }
  ],
  "total": 3,
  "has_more": false
}
```

---

## 5. Query Pipeline — No Changes Required

The existing `context_filter::filter_context_by_document_ids` is already correct:

```
filter_context_by_document_ids(&mut context, Option<&[String]>)
  - None → no-op (all workspace)
  - Some([]) → empty context (nothing matches)
  - Some(["a","b"]) → filter chunks to those with document_id in {"a","b"}
```

The `document_filter_resolver.rs` changes above ensure that the IDs fed into this function are correct. The engine itself requires zero changes.

### 5.1 Pipeline Data Flow (Updated)

```
POST /api/v1/query { document_filter: { document_ids: ["a","b"] } }
  |
  v
query_execute.rs:
  if let Some(filter) = &request.document_filter {
      let ids = resolve_document_filter(kv, filter, tenant_id, workspace_id).await?;
      if let Some(allowed) = ids {
          engine_request = engine_request.with_allowed_document_ids(allowed);
      }
  }
  |
  v  (fast path: document_ids only, no date/pattern)
document_filter_resolver.rs:
  has_explicit_ids=true, !has_pattern, !has_date_filter
  → return Some(["a","b"]) immediately (NO KV scan)
  |
  v
QueryEngine.execute(request { allowed_document_ids: Some(["a","b"]) })
  → context_filter::filter_context_by_document_ids (post-retrieval)
  → returns filtered context
```

**Performance note**: When only `document_ids` are set (the common case from the UI picker), the KV scan is completely bypassed. This is a significant improvement over the current pattern-based flow.

---

## 6. RAG Engine Behavior with Explicit Scope

### 6.1 Graph Query Impact

The `allowed_document_ids` filter in `context_filter.rs` applies to:
- **Chunk retrieval**: Only chunks from allowed documents are used
- **Entity retrieval (local mode)**: Entities from disallowed documents are excluded
- **Graph traversal (global mode)**: Entities/relationships only from allowed documents
- **Hybrid/mix mode**: All sub-modes are filtered

This means **explicit document scope propagates through all RAG retrieval modes**. A query scoped to 2 documents in "global" mode will only traverse entities extracted from those 2 documents.

### 6.2 Implication for "Global" Mode

In global mode, the knowledge graph is traversed across all entities. When scope is restricted to 2 documents, the graph slice for those documents may have fewer cross-connections, resulting in less "global" reasoning. This is **expected behavior** — the user explicitly restricted scope.

The UI should convey this: "Your query is scoped to 2 documents. Global knowledge graph connections may be reduced."

---

## 7. OpenAPI Documentation Update

The new `document_ids` field must appear in OpenAPI schema:

```yaml
DocumentFilter:
  type: object
  properties:
    date_from:
      type: string
      format: date-time
      description: "Start date filter (ISO 8601)"
    date_to:
      type: string
      format: date-time
      description: "End date filter (ISO 8601)"
    document_pattern:
      type: string
      description: "Case-insensitive title substring. Comma-separated = OR."
    document_ids:
      type: array
      items:
        type: string
        format: uuid
      description: >
        Explicit document IDs to scope the query.
        Union with document_pattern if both set.
        Empty array treated as null (no filtering).
```

---

## 8. Migration & Backward Compatibility

| Consumer                            | Impact                                                 | Action        |
| ----------------------------------- | ------------------------------------------------------ | ------------- |
| Existing SPEC-005 clients           | None — `document_ids` is optional                      | None          |
| `query_types.rs` `DocumentFilter`   | Additive field with `#[serde(default)]`                | None          |
| `document_filter_resolver.rs` tests | Existing tests still pass (None behavior unchanged)    | Add new tests |
| Chat completions (`chat_types.rs`)  | `DocumentFilter` reused — gets new field automatically | Test          |
| Streaming queries                   | Same resolver path — no change                         | Test          |
