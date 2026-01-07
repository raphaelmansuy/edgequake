# Iteration 32 - Orient

**Date:** 2026-01-07  
**Focus:** query.rs DTO extraction strategy

## Pattern Application

### Established Pattern (7th Extraction)
```
handlers/
  query.rs          (handler logic + re-exports)
  query_types.rs    (DTOs + helpers + tests)
  mod.rs            (module exports)
```

### Extraction Strategy

**Phase 1: Create query_types.rs**
- Extract 6 DTOs with full documentation
- Include default_enable_rerank() helper
- Add 10 comprehensive unit tests
- Validate serialization/deserialization

**Phase 2: Update Module System**
- Add query_types to mod.rs
- Re-export via `pub use query_types::*;`

**Phase 3: Refactor query.rs**
- Replace inline DTOs with imports
- Add `pub use` for backward compatibility
- Maintain handler logic unchanged

**Phase 4: Validation**
- Build verification
- Run full test suite
- Verify 261 tests passing

## Key Decisions

### 1. DTO Grouping
All 6 DTOs belong together:
- QueryRequest/QueryResponse: Core request/response
- ConversationMessage: Multi-turn context
- SourceReference/QueryStats: Response metadata
- StreamQueryRequest: Streaming variant

### 2. Backward Compatibility
Must re-export from query.rs:
```rust
pub use crate::handlers::query_types::{
    ConversationMessage, QueryRequest, QueryResponse,
    QueryStats, SourceReference, StreamQueryRequest,
};
```

Reason: chat.rs and chat_types.rs import from query module

### 3. Test Design
- Test minimal QueryRequest (defaults)
- Test full QueryRequest (all options)
- Test ConversationMessage serialization
- Test StreamQueryRequest deserialization
- Test SourceReference with/without optional fields
- Test QueryStats with/without rerank_time_ms
- Test QueryResponse serialization
- Use epsilon comparisons for f32/f64 fields

### 4. Helper Function Location
Place default_enable_rerank() in query_types.rs:
- Tightly coupled to QueryRequest
- Used in serde default attribute
- Better cohesion with DTO

## Expected Outcomes

**Code Quality:**
- query.rs: 588 → ~430 lines (-27%)
- New query_types.rs: ~320 lines
- 10 new unit tests
- Clean module separation

**Validation:**
- All 261 tests passing
- No breaking changes
- Backward compatible imports

**Modularity:**
- Single responsibility maintained
- DTOs separated from handler logic
- Consistent with previous 6 extractions
