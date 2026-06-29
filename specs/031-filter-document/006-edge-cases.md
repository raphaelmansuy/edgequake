# SPEC-031 / 006 — Edge Cases & Mitigations

> **Lens**: Engineering (defensive design)  
> **Principle**: Fail safe · Explicit over implicit · Zero silent errors

---

## 1. Edge Cases Matrix

### 1.1 Input Validation

| Edge Case | Scenario                                      | Mitigation                                          | Behavior                                    |
| --------- | --------------------------------------------- | --------------------------------------------------- | ------------------------------------------- |
| EC-01     | `document_ids: []` (empty array)              | `DocumentFilter.is_empty()` checks `ids.is_empty()` | Treated as `null` — no filtering            |
| EC-02     | `document_ids` contains duplicates            | `deduplicate()` in resolver                         | Deduped before matching                     |
| EC-03     | `document_ids` contains non-existent IDs      | Resolver does KV scan; absent IDs not found         | Silently ignored, logged at DEBUG           |
| EC-04     | `document_ids` with 0 valid IDs (all phantom) | Resolver returns `Some(vec![])`                     | Empty result set — no context, no answer    |
| EC-05     | `document_ids` > 100 items                    | Server-side cap enforced in endpoint                | HTTP 400: "Too many document IDs (max 100)" |
| EC-06     | `document_ids` contains SQL injection / XSS   | IDs are UUID strings, validated by storage layer    | Invalid UUIDs return no match               |
| EC-07     | `q` search param is empty string              | Treated same as absent `q`                          | Returns 20 most recent documents            |
| EC-08     | `q` search param > 200 chars                  | Server truncates to 200 chars                       | Search still executes on truncated input    |

### 1.2 Concurrency & State

| Edge Case | Scenario                                        | Mitigation                                      | Behavior                                                              |
| --------- | ----------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------- |
| EC-09     | Document deleted while it's in scope selection  | Resolver returns `Some(vec![])` or partial      | Missing doc silently excluded; context still valid for remaining docs |
| EC-10     | Document re-ingested (same ID) while query runs | Engine uses stale context window                | RAG query completes; next query sees fresh data                       |
| EC-11     | Scope changes while query is streaming          | Scope is captured at query start                | In-flight query uses original scope; new scope applies next query     |
| EC-12     | User closes browser mid-selection               | `scopedDocumentIds` persisted in `localStorage` | Selection restored on next visit                                      |
| EC-13     | Page refresh during active scope                | `useQuerySettings` restores from `localStorage` | Pills re-render with previous selection                               |

### 1.3 Network & API

| Edge Case | Scenario                                      | Mitigation                 | Behavior                                               |
| --------- | --------------------------------------------- | -------------------------- | ------------------------------------------------------ |
| EC-14     | `GET /documents/search` returns 500           | React Query error state    | Picker shows "Unable to load documents" + retry button |
| EC-15     | `GET /documents/search` returns 401           | API client interceptor     | Redirect to login (same as all authenticated routes)   |
| EC-16     | Search timeout (> 5s)                         | React Query timeout config | Stale data shown (placeholderData) or error state      |
| EC-17     | Very fast typing (< 300ms between keystrokes) | Debounce at 300ms          | Only sends one request per 300ms idle                  |
| EC-18     | Race condition: fast typing + slow response   | React Query deduplication  | Only latest request is used (stale previous discarded) |

### 1.4 Document Status Edge Cases

| Edge Case | Scenario                                                             | Mitigation                                          | Behavior                                                                          |
| --------- | -------------------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------- |
| EC-19     | User selects a `processing` document (via pattern filter)            | Picker default shows only `completed`               | Search endpoint defaults to `status=completed`; explicit selection still possible |
| EC-20     | User explicitly passes ID of a `failed` document                     | No status check at query level                      | Query executes; no chunks found from that document (effectively empty)            |
| EC-21     | Document transitions from `processing` to `completed` while in scope | No change — scope is ID-based                       | Next query (after completion) will find chunks                                    |
| EC-22     | All scoped documents are `failed` (0 chunks)                         | Resolver returns `Some(ids)`, engine finds 0 chunks | LLM receives empty context; response indicates insufficient information           |

### 1.5 UX Edge Cases

| Edge Case | Scenario                                          | Mitigation                                     | Behavior                                             |
| --------- | ------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------- |
| EC-23     | `scopedDocumentIds` has IDs with no cached titles | `useDocumentTitle` returns undefined           | Pill shows truncated UUID (first 8 chars + ellipsis) |
| EC-24     | More than 3 pills visible                         | Truncate to 3 + "+N more" chip                 | Click "+N more" to see all or open picker            |
| EC-25     | Very long document title (> 200 chars)            | Truncate to 22 chars in pill                   | Full title in `title` tooltip attribute              |
| EC-26     | Workspace has 0 documents                         | Search endpoint returns empty `items`          | Picker shows "No completed documents yet"            |
| EC-27     | Workspace has 10,000+ documents, no search query  | Search endpoint defaults to `page_size=20`     | User sees most recent 20; search narrows             |
| EC-28     | Scope bar overflows horizontally on mobile        | `overflow-x-auto scrollbar-none` on pills list | Pills scroll, no wrapping or clipping                |
| EC-33     | User doesn't notice "All docs ▾" button           | Button is always present above textarea        | Low-contrast by design; tooltip explains on hover    |

### 1.6 Interaction with SPEC-005 Filters

| Edge Case | Scenario                                                                    | Mitigation                                       | Behavior                                                                   |
| --------- | --------------------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------- |
| EC-29     | Both `document_ids` and `document_pattern` set                              | Resolver unions both sets                        | A doc matches if it's in IDs OR matches pattern (then date filter applied) |
| EC-30     | `document_ids = ["a"]` + `date_from` set but doc "a" has no `created_at`    | `passes_date_filter` returns false + warns       | Doc "a" excluded; behavior same as today for docs without timestamps       |
| EC-31     | All filters set: `document_ids`, `document_pattern`, `date_from`, `date_to` | Full resolve path executes                       | KV scan, union ids+pattern, AND date filter                                |
| EC-32     | `document_ids = null` + `document_pattern = "report"`                       | `has_explicit_ids = false`, `has_pattern = true` | Pattern-only filter (backward compatible)                                  |

---

## 2. Invariants (Always-True Assertions)

The system must uphold these invariants at all times:

### INV-01: Default is All Workspace
When `DocumentFilter` is absent OR `DocumentFilter.is_empty() == true`, the query pipeline receives `allowed_document_ids = None`, which means "all documents". No filtering occurs.

### INV-02: Empty Array = No Filter
`document_ids: []` must never trigger an empty-result short-circuit. It must be treated identically to `document_ids: null`.

**Enforcement**: `DocumentFilter.is_empty()` checks `ids.is_empty()`.

### INV-03: Resolver Never Panics
The resolver handles all invalid inputs by returning `Ok(None)` or `Ok(Some(vec![]))` — never `Err(...)` except for genuine storage failures.

### INV-04: Scope Toolbar Always Renders
The `QueryScopeBar` component **always renders** — it has no null-return guard.
In the empty state it shows an "All docs ▾" trigger button. This ensures the
feature is discoverable without any prior interaction.

The distinction is purely visual:
- Empty: muted ghost button, no background tint
- Active: secondary pills, `bg-muted/40` background

### INV-05: Scope Persists Across Sessions (with Explicit Clear)
`scopedDocumentIds` is persisted in `localStorage`. Users must explicitly clear scope. This prevents the frustration of losing a carefully assembled scope on page refresh.

### INV-06: Picker Shows Only Completed Documents by Default
The `DocumentPickerPopover` defaults to `status=completed` to prevent users from inadvertently scoping to broken documents. Advanced users can use `document_pattern` (SPEC-005) or MCP with `status=all` to override.

### INV-07: Query Scope Does Not Affect Conversation History
Scope changes only affect new queries, not previous messages in the conversation. A message says "Scoped to 2 documents" and subsequent messages without scope change use the same scope implicitly (per `useQuerySettings`).

---

## 3. Security Considerations

### SEC-01: Tenant Isolation
The `search_documents` endpoint enforces tenant context (`has_full_tenant_context()`) exactly like `list_documents`. An empty tenant context returns an empty result, not a 401. This is consistent with the security model.

### SEC-02: No Cross-Workspace ID Leakage
The `document_filter_resolver.rs` always filters via scoped metadata SSOT (`load_scoped_document_metadata_entries`). Even if a client sends IDs from another workspace, those IDs are not found in the current workspace's KV scan and are silently ignored.

**Invariant**: A document ID from workspace A can never influence query results in workspace B.

### SEC-03: Input Sanitization
- `q` parameter: truncated to 200 chars, no SQL injection risk (used as substring match on in-memory strings from KV)
- `page_size`: server caps at 50, input cast to `usize`
- `document_ids`: treated as opaque strings; no special characters affect the KV lookup path (UUIDs are compared by string equality)

### SEC-04: Rate Limiting
The `search_documents` endpoint does not have dedicated rate limiting beyond the existing server middleware. At scale (10,000+ documents), KV scan takes ~50-100ms. The 300ms debounce on the client reduces server load to manageable levels. If needed, a future spec can add Redis-backed document title caching.

---

## 4. Monitoring & Observability

Add the following structured log events:

```rust
// In search_documents handler:
info!(
    workspace_id = ?tenant_ctx.workspace_id,
    query = ?params.q,
    result_count = items.len(),
    "Document search executed"
);

// In document_filter_resolver.rs:
debug!(
    has_explicit_ids,
    has_pattern,
    has_date_filter,
    matched_count = matched_ids.len(),
    "Document filter resolved"
);

// When explicit-only path taken (no KV scan):
debug!(
    explicit_id_count = ids.len(),
    "Document filter: explicit IDs only, skipping KV scan"
);
```

---

## 5. Testing Requirements

### 5.1 Backend Unit Tests

| Test                                    | Assertion                                                         |
| --------------------------------------- | ----------------------------------------------------------------- |
| `test_empty_document_ids_is_noop`       | `document_ids: Some(vec![])` → returns `None`                     |
| `test_explicit_ids_skip_kv_scan`        | No pattern/date + IDs → returns IDs immediately (KV never called) |
| `test_explicit_ids_with_date_filter`    | IDs + date_from → KV scan, only matching doc returned             |
| `test_ids_union_pattern`                | IDs + pattern → union of both sets                                |
| `test_nonexistent_ids_silently_ignored` | IDs not in KV → empty result                                      |
| `test_search_endpoint_empty_query`      | `q=""` → returns 20 most recent completed docs                    |
| `test_search_endpoint_status_filter`    | `status=all` → returns all statuses                               |
| `test_search_endpoint_page_size_cap`    | `page_size=1000` → returns max 50                                 |

### 5.2 Frontend Unit Tests

| Test                                          | Assertion                                              |
| --------------------------------------------- | ------------------------------------------------------ |
| `QueryScopeBar renders null when empty`       | `selectedIds=[]` → no DOM nodes                        |
| `QueryScopeBar shows pills`                   | `selectedIds=["a","b"]` → 2 pill elements              |
| `QueryScopeBar shows +N for overflow`         | `selectedIds=[1,2,3,4]` → 3 pills + "+1"               |
| `ScopePill remove button fires callback`      | Click `×` → `onSelectionChange` called without that ID |
| `DocumentPickerPopover checkboxes toggle IDs` | Click item → ID added/removed                          |
| `useDocumentSearch debounces 300ms`           | Rapid typing → single network request after delay      |
| `isEmptyDocumentFilter`                       | All cases: null, empty obj, partial, full              |
