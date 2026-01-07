# Iteration 26 - Decide

## Actions to Take

### Phase 1: Create documents_types.rs
1. Create new file with all 22 DTO structs
2. Include all helper functions for serde defaults
3. Add comprehensive unit tests for serialization and defaults
4. Add proper imports (serde, utoipa)

### Phase 2: Update handlers/mod.rs
1. Add `pub mod documents_types;` declaration
2. Add `pub use documents_types::*;` export

### Phase 3: Update documents.rs
1. Remove inline DTOs (22 structs)
2. Remove helper functions (10+ functions)
3. Add `pub use crate::handlers::documents_types::*;` import
4. Remove unused imports (serde::Deserialize, serde::Serialize, utoipa::ToSchema)

### Phase 4: Validation
1. Run `cargo build --package edgequake-api`
2. Run `cargo test --package edgequake-api --lib`
3. Verify 188+ tests pass

## Success Criteria
- [ ] documents.rs reduced by ~600+ lines
- [ ] documents_types.rs contains all 22 DTOs
- [ ] All 188+ tests pass
- [ ] No duplicate exports or ambiguous types

## Rollback Plan
If tests fail:
1. Revert documents.rs to pre-edit state
2. Delete documents_types.rs
3. Update mod.rs to remove documents_types references
4. Document failure in act.md
