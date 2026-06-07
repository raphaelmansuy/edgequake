# SPEC-006 E2E Proof 004 — Graph Timeout No Fallback

**Covers:** BR-006-014, V-006-003  
**Test:** `resource_safety_graph_query_timeout_response`

## Assertion

When graph popular-nodes query exceeds budget:

- HTTP **503** `SERVICE_UNAVAILABLE`
- Header **`Retry-After: 30`**
- **No** `get_all_nodes()` fallback (`traversal.rs` removed fallback block)

## AX-02 compliance

Fallback must never be more expensive than failure.

## Run

```bash
cargo test -p edgequake-api resource_safety_graph_query_timeout
```

## Code is law

- `handlers/graph/graph_query/traversal.rs` — returns `ApiError::graph_query_timeout()`
- `error.rs` — `ServiceUnavailable { retry_after_secs: 30 }`
