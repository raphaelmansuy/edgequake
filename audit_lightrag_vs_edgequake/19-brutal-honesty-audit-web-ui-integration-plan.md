# BRUTAL HONESTY AUDIT: SOTA Query Engine Integration Status

> **Date:** 2025-01-01  
> **Status:** 🔴 **CRITICAL GAP IDENTIFIED**  
> **Summary:** SOTA engine is implemented but NOT used anywhere in production

---

## Executive Summary

**The Problem:** The `SOTAQueryEngine` was created as a parallel implementation alongside the existing `QueryEngine`. The API layer and web UI continue to use the OLD `QueryEngine`, meaning:

- ❌ **Users are NOT getting SOTA GraphRAG retrieval**
- ❌ **LLM keyword extraction is NOT active**
- ❌ **Adaptive mode selection is NOT available**
- ❌ **High/Low level keyword embeddings are NOT being computed**

**The Fix Required:**

1. Replace `QueryEngine` with `SOTAQueryEngine` in API layer
2. Verify web UI works with the new response format (should be compatible)
3. Add integration tests for the complete flow

---

## Territory Mapping: Current Architecture

### 1. Query Flow Analysis

```
Current Flow (BROKEN):
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web UI        │ -> │  API Layer      │ -> │  OLD QueryEngine│
│   (React/TS)    │    │  (Axum/Rust)    │    │  (engine.rs)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                       Uses state.query_engine    NO LLM keywords
                       Arc<QueryEngine>           NO adaptive mode

Desired Flow (SOTA):
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────┐
│   Web UI        │ -> │  API Layer      │ -> │  SOTAQueryEngine    │
│   (React/TS)    │    │  (Axum/Rust)    │    │  (sota_engine.rs)   │
└─────────────────┘    └─────────────────┘    └─────────────────────┘
                       Uses state.sota_engine    ✅ LLM keywords
                       Arc<SOTAQueryEngine>      ✅ Adaptive mode
                                                 ✅ VectorType filtering
```

### 2. Files Using OLD QueryEngine (Must be Modified)

| File                                  | Line | Usage                                                   | Action Required            |
| ------------------------------------- | ---- | ------------------------------------------------------- | -------------------------- |
| `edgequake-api/src/state.rs`          | 13   | `use edgequake_query::{QueryEngine, QueryEngineConfig}` | Add SOTAQueryEngine import |
| `edgequake-api/src/state.rs`          | 88   | `pub query_engine: Arc<QueryEngine>`                    | Add `sota_engine` field    |
| `edgequake-api/src/state.rs`          | 243  | `QueryEngine::new(...)`                                 | Create SOTAQueryEngine     |
| `edgequake-api/src/state.rs`          | 309  | `QueryEngine::new(...)` (postgres)                      | Create SOTAQueryEngine     |
| `edgequake-api/src/state.rs`          | 475  | `QueryEngine::new(...)` (test)                          | Create SOTAQueryEngine     |
| `edgequake-api/src/handlers/query.rs` | 11   | Uses `QueryMode`                                        | Compatible ✅              |
| `edgequake-api/src/handlers/query.rs` | 180+ | `state.query_engine.query(...)`                         | Use `state.sota_engine`    |
| `edgequake-api/src/handlers/chat.rs`  | 397  | `.query_engine.query(...)`                              | Use `state.sota_engine`    |
| `edgequake-api/src/handlers/chat.rs`  | 645  | `.query_engine.query_stream(...)`                       | Need streaming support     |

### 3. API Request/Response Compatibility

**GOOD NEWS:** The `SOTAQueryEngine::query()` method returns `crate::engine::QueryResponse`, which is the SAME type as `QueryEngine::query()`. This means:

- ✅ Web UI response parsing is compatible
- ✅ API handlers can switch engines without changing response format
- ✅ No frontend changes required

---

## SOTA Engine Capabilities (What's Being Wasted)

| Feature                     | Status         | Currently Used? |
| --------------------------- | -------------- | --------------- |
| LLM Keyword Extraction      | ✅ Implemented | ❌ NO           |
| High/Low Level Keywords     | ✅ Implemented | ❌ NO           |
| Query Intent Classification | ✅ Implemented | ❌ NO           |
| Adaptive Mode Selection     | ✅ Implemented | ❌ NO           |
| VectorType Filtering        | ✅ Implemented | ❌ NO           |
| Batch Graph Operations      | ✅ Implemented | ❌ NO           |
| Keyword Caching             | ✅ Implemented | ❌ NO           |

**All 26 SOTA tests pass, but the engine is orphaned!**

---

## Critical Gap: Streaming Support

**Problem:** `SOTAQueryEngine` does NOT implement `query_stream()`.

The chat handler at line 645 calls:

```rust
state_clone.query_engine.query_stream(engine_request).await
```

**Impact:** Streaming chat will break if we simply swap engines.

**Solution Options:**

1. **Add `query_stream()` to SOTAQueryEngine** (Recommended)
   - Copy streaming logic from old `QueryEngine`
   - Apply SOTA enhancements to streaming path
2. **Keep both engines, route by streaming** (Hacky)
   - Use SOTA for non-streaming, old for streaming
   - Leads to inconsistent behavior

---

## Implementation Plan

### Phase 1: Add Streaming to SOTAQueryEngine (1-2 hours)

**File:** `edgequake-query/src/sota_engine.rs`

Add `query_stream()` method that:

1. Extracts keywords with caching
2. Computes QueryEmbeddings
3. Performs mode-specific retrieval
4. Streams LLM response

### Phase 2: Modify API State (30 mins)

**File:** `edgequake-api/src/state.rs`

1. Add import: `use edgequake_query::{SOTAQueryEngine, SOTAQueryConfig};`
2. Add field: `pub sota_engine: Arc<SOTAQueryEngine>`
3. Create SOTAQueryEngine in all constructors

### Phase 3: Update Handlers (30 mins)

**Files:**

- `edgequake-api/src/handlers/query.rs`
- `edgequake-api/src/handlers/chat.rs`

Replace all `state.query_engine` with `state.sota_engine`

### Phase 4: Integration Tests (1 hour)

**File:** `edgequake-api/tests/e2e_sota_integration.rs`

Test complete flow:

1. Upload document → entities extracted
2. Query with SOTA engine → get LightRAG-style results
3. Streaming chat → works with new engine
4. Verify keyword caching works

### Phase 5: Web UI Verification (30 mins)

- Start backend with SOTA engine
- Verify query page works
- Verify chat page works
- Check console for errors

---

## Files to Create/Modify

### Create:

- [ ] `edgequake-api/tests/e2e_sota_integration.rs` - Integration tests

### Modify:

- [ ] `edgequake-query/src/sota_engine.rs` - Add `query_stream()`
- [ ] `edgequake-api/src/state.rs` - Add SOTAQueryEngine
- [ ] `edgequake-api/src/handlers/query.rs` - Use sota_engine
- [ ] `edgequake-api/src/handlers/chat.rs` - Use sota_engine

---

## Web UI Analysis

### Files That Call Query API:

| File                                | Purpose            | SOTA Impact      |
| ----------------------------------- | ------------------ | ---------------- |
| `src/lib/api/edgequake.ts`          | `query()` function | No change needed |
| `src/lib/api/chat.ts`               | `chatCompletion()` | No change needed |
| `src/hooks/use-query-page-state.ts` | Query page state   | No change needed |

### Response Format Compatibility:

The API returns `QueryResponse` with:

- `answer: String`
- `mode: String`
- `sources: Vec<SourceReference>`
- `stats: QueryStats`

**This format is unchanged between old and SOTA engine.**

---

## Risk Assessment

| Risk                   | Impact | Mitigation                                                     |
| ---------------------- | ------ | -------------------------------------------------------------- |
| Streaming breaks       | HIGH   | Implement query_stream() first                                 |
| Performance regression | MEDIUM | SOTA may be slower due to LLM keyword extraction. Add caching. |
| API breaking change    | LOW    | Response format is compatible                                  |
| Web UI breaks          | LOW    | No changes to response format                                  |

---

## Success Criteria

1. ✅ All 1332 workspace tests pass
2. ✅ SOTA engine used for all queries
3. ✅ LLM keyword extraction active (verifiable in logs)
4. ✅ Web UI query page works
5. ✅ Web UI chat page with streaming works
6. ✅ Integration tests cover full flow

---

## Estimated Timeline

| Phase     | Time      | Description                      |
| --------- | --------- | -------------------------------- |
| 1         | 2h        | Add streaming to SOTAQueryEngine |
| 2         | 30m       | Update API state                 |
| 3         | 30m       | Update handlers                  |
| 4         | 1h        | Integration tests                |
| 5         | 30m       | Web UI verification              |
| **Total** | **~4.5h** | Complete integration             |

---

## Post-SOTA Roadmap Preview

Once SOTA is integrated:

1. **Source ID Tracking**: Link entities back to source chunks for citations
2. **Token Budgeting**: Dynamic allocation based on mode
3. **Query Caching**: Cache complete results with intelligent invalidation
4. **Reranking**: Cross-encoder for improved relevance
5. **Streaming Improvements**: Progressive context disclosure
6. **Analytics Dashboard**: Query mode distribution, keyword hit rates

---

## Appendix: Code Evidence

### SOTAQueryEngine NOT in API

```bash
$ grep -r "SOTAQueryEngine" edgequake/crates/edgequake-api/
# Returns: NO MATCHES
```

### QueryEngine IS in API

```bash
$ grep -r "query_engine" edgequake/crates/edgequake-api/src/
state.rs:    pub query_engine: Arc<QueryEngine>,
state.rs:        query_engine: Arc<QueryEngine>,
# ... 15+ more matches
```

### SOTA Engine Exports Available

```rust
// edgequake-query/src/lib.rs line 52
pub use sota_engine::{QueryEmbeddings, SOTAQueryConfig, SOTAQueryEngine};
```

**Conclusion:** SOTA engine is ready for integration but completely disconnected from production.
