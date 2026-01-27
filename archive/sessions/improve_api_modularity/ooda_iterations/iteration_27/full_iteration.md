# Iteration 27: Extract DTOs from graph.rs to graph_types.rs

## OBSERVE

**Target**: `edgequake/crates/edgequake-api/src/handlers/graph.rs`

- Initial size: 1,150 lines
- Contains: 13 DTO structs/enums inline with handlers
- Pattern established: Sibling file approach from iteration 26

**DTOs identified**:

1. GraphNodeResponse
2. GraphEdgeResponse
3. KnowledgeGraphResponse
4. GraphQueryParams
5. SearchLabelsQuery
6. SearchLabelsResponse
7. PopularLabelsQuery
8. PopularLabel
9. PopularLabelsResponse
10. BatchDegreeRequest
11. NodeDegree
12. BatchDegreeResponse
13. GraphStreamQueryParams + GraphStreamEvent (includes helper functions)

## ORIENT

**Learnings from iteration 26**:

- Sibling `*_types.rs` file approach works correctly
- Re-export via `pub use crate::handlers::*_types::*;` maintains API compatibility
- Adding unit tests for DTOs increases test count meaningfully (201→210)

**Strategy**:

- Create `graph_types.rs` as sibling to `graph.rs`
- Include helper functions (`default_stream_max_nodes`, `default_stream_batch_size`)
- Add serialization/defaults tests for key DTOs

## DECIDE

1. Create `graph_types.rs` with all 13 DTOs + helper functions + tests
2. Update `handlers/mod.rs` to export new module
3. Add `pub use crate::handlers::graph_types::*;` to graph.rs
4. Remove all inline DTO definitions from graph.rs
5. Run tests to validate non-regression

## ACT

### Changes Made

**Created**: `graph_types.rs` (~350 lines)

- 13 DTOs with full utoipa annotations
- 2 helper functions for stream defaults
- 9 unit tests for serialization and defaults

**Modified**: `handlers/mod.rs`

- Added `pub mod graph_types;`
- Added `pub use graph_types::*;`

**Modified**: `graph.rs`

- Added `pub use crate::handlers::graph_types::*;` at top
- Removed all 13 inline DTO definitions
- Reduced from 1,150 to 896 lines (-254 lines, -22%)

### Validation

```
cargo test --package edgequake-api --lib
test result: ok. 210 passed; 0 failed; 0 ignored
```

### Commit

```
f2a8172 refactor(api): Extract 13 DTOs from graph.rs to graph_types.rs
```

## METRICS

| Metric         | Before | After | Change      |
| -------------- | ------ | ----- | ----------- |
| graph.rs lines | 1,150  | 896   | -254 (-22%) |
| graph_types.rs | 0      | ~350  | +350        |
| API lib tests  | 201    | 210   | +9          |
| DTOs extracted | 0      | 13    | +13         |

## NEXT

Continue to iteration 28: Apply same pattern to conversations.rs (1,197 lines)
