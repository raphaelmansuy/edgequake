# Iteration 32 - Decide

**Date:** 2026-01-07  
**Focus:** query.rs DTO extraction execution plan

## Implementation Plan

### Step 1: Create query_types.rs ✅

```rust
// Structure:
// - Module documentation
// - Helper functions (default_enable_rerank)
// - Request DTOs (ConversationMessage, QueryRequest, StreamQueryRequest)
// - Response DTOs (QueryResponse, SourceReference, QueryStats)
// - Unit tests (10 tests covering all DTOs)
```

**Validation:** File compiles with all required traits

### Step 2: Update mod.rs ✅

```rust
// Add:
pub mod query_types;
pub use query_types::*;
```

**Validation:** Module exports correctly

### Step 3: Refactor query.rs ✅

```rust
// Replace inline DTOs with:
pub use crate::handlers::query_types::{
    ConversationMessage, QueryRequest, QueryResponse,
    QueryStats, SourceReference, StreamQueryRequest,
};
```

**Validation:** File compiles without DTO definitions

### Step 4: Run Tests ✅

```bash
cargo test --package edgequake-api --lib
```

**Expected:** 261 tests passing (252 + 9 new)

### Step 5: Fix Test Issues ✅

**Issue:** Floating point comparison failed
**Fix:** Use epsilon comparison instead of equality

```rust
// Before:
assert_eq!(json["score"], 0.95);

// After:
let score = json["score"].as_f64().unwrap();
assert!((score - 0.95).abs() < 0.01);
```

**Validation:** All tests passing

### Step 6: Commit Changes ✅

```bash
git add -A
git commit -m "refactor(api): Extract 6 DTOs from query.rs to query_types.rs (iteration 32)"
```

**Validation:** Clean commit with meaningful message

## Success Criteria

- [x] query_types.rs created with 6 DTOs
- [x] default_enable_rerank() helper included
- [x] 10 unit tests added
- [x] mod.rs updated with exports
- [x] query.rs refactored with re-exports
- [x] 261 tests passing
- [x] No breaking changes
- [x] Committed as c35cb55

## Risk Mitigation

**Risk:** Breaking backward compatibility  
**Mitigation:** Re-export all types from query.rs ✅

**Risk:** Test failures due to floating point precision  
**Mitigation:** Use epsilon comparisons for f32/f64 ✅

**Risk:** Missing imports in dependent modules  
**Mitigation:** Maintain `pub use` re-exports ✅

## Next Steps

Continue to iteration 33:

- Target: tasks.rs (475 lines)
- Identify DTOs for extraction
- Follow established pattern
