# OODA Loop 3: Orient

## Options Analysis

### Option A: Default to Enhanced (Breaking Change)
Change `BM25Reranker::new()` to use enhanced tokenization.

**Pros**: Immediate benefit for all users
**Cons**: Breaking change, could affect test expectations

### Option B: Factory Function with Environment Variable
Create helper function that reads `BM25_ENHANCED` env var.

**Pros**: 
- Non-breaking (defaults to enhanced but can disable)
- Centralized logic
- Tracing integration for observability

**Cons**: Environment variable coupling

### Option C: Configuration Struct
Add `BM25Config` to API configuration.

**Pros**: Type-safe, explicit
**Cons**: Requires schema changes, more invasive

## Selected Approach: Option B

Create `create_bm25_reranker()` helper function that:
1. Checks `BM25_ENHANCED` environment variable
2. Defaults to enhanced (new behavior)
3. Falls back to minimal if `BM25_ENHANCED=false`
4. Logs which mode is active

This allows:
- Immediate benefit for new deployments
- Easy rollback for production issues
- Observability through tracing
