# OODA Loop 3: Decide

## Decisions

### D1: Create Helper Function
Add `create_bm25_reranker()` at module level in `state.rs`.

### D2: Default to Enhanced
New deployments get stemming + Unicode + stop words by default.
Existing deployments can set `BM25_ENHANCED=false` to preserve behavior.

### D3: Add Tracing
Log which mode is active at startup for observability.

### D4: Update Both Constructors
- `new_memory()`: Use helper
- `new_postgres()`: Use helper

### D5: Document Environment Variable
Add `BM25_ENHANCED` to `.env.example` with explanation.

## Risk Mitigation
- Environment variable check happens at runtime
- `BM25_ENHANCED=false` provides immediate rollback
- All tests pass with enhanced tokenizer
