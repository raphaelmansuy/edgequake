# Iteration 25: Decide

## Implementation Plan

### Goal

Refactor [documents.rs](../../../edgequake/crates/edgequake-api/src/handlers/documents.rs) (3,573 lines) into focused modules following Single Responsibility Principle.

### Target Structure

```
handlers/documents/
├── mod.rs          # Public API re-exports
├── dtos.rs         # All request/response types
├── upload.rs       # Text document upload
├── list.rs         # Document listing
├── detail.rs       # Document detail retrieval
├── delete.rs       # Deletion + impact analysis
├── files.rs        # File upload (single)
└── batch.rs        # Batch file upload
```

### Step-by-Step Actions

#### Step 1: Create Module Structure

```bash
mkdir -p edgequake/crates/edgequake-api/src/handlers/documents
touch edgequake/crates/edgequake-api/src/handlers/documents/{mod.rs,dtos.rs}
```

#### Step 2: Extract DTOs (~500 lines)

**Target**: `dtos.rs`

Move these types from `documents.rs`:

- `UploadDocumentRequest` (lines 19-55)
- `UploadDocumentResponse` (lines 66-102)
- `DocumentCostInfo` (lines 103-138)
- `ListDocumentsRequest` (lines 506-525)
- `StatusCounts` (lines 526-538)
- `ListDocumentsResponse` (lines 539-557)
- `DocumentSummary` (lines 558-638)
- `GetDocumentRequest` (lines 958-964)
- `DocumentDetailResponse` (lines 965-1056)
- `DocumentLineage` (lines 1057-1124)
- `DeleteDocumentResponse` (lines 1443-1472)
- `DeletionImpactResponse` (lines 1615-1654)
- `FileUploadResponse` (lines 1742-1785)
- `BatchUploadResponse` (lines 2190-2208)
- `BatchFileResult` (lines 2209-2248)

**Action**: Create `dtos.rs`, move types, add necessary imports.

#### Step 3: Create mod.rs with Re-exports

```rust
//! Document ingestion handlers.

pub mod dtos;
// Future modules will be added here

// Re-export DTOs
pub use dtos::*;

// Re-export handlers (will be added incrementally)
```

**Test Command**: `cargo test -p edgequake-api documents`

#### Step 4: Update documents.rs Imports

Replace DTO definitions with:

```rust
use super::documents::dtos::*;
```

**Test Command**: `cargo test -p edgequake-api documents`
**Commit**: `refactor(api): Extract documents DTOs to separate module`

#### Step 5: Extract Upload Handler (~400 lines)

**Target**: `upload.rs`

Move from `documents.rs`:

- `upload_document` function (lines 140-505)
- Helper functions used by upload

**Test Command**: `cargo test -p edgequake-api upload`
**Commit**: `refactor(api): Extract upload_document handler to documents/upload.rs`

#### Step 6: Extract List Handler (~300 lines)

**Target**: `list.rs`

Move from `documents.rs`:

- `list_documents` function (lines 639-957)
- Associated helpers

**Test Command**: `cargo test -p edgequake-api list_documents`
**Commit**: `refactor(api): Extract list_documents handler to documents/list.rs`

#### Step 7: Extract Detail Handler (~280 lines)

**Target**: `detail.rs`

Move from `documents.rs`:

- `get_document` function (lines 1125-1442)
- Associated helpers

**Test Command**: `cargo test -p edgequake-api get_document`
**Commit**: `refactor(api): Extract get_document handler to documents/detail.rs`

#### Step 8: Extract Delete Handlers (~300 lines)

**Target**: `delete.rs`

Move from `documents.rs`:

- `delete_document` function (lines 1473-1614)
- `analyze_deletion_impact` function (lines 1655-1741)
- Associated helpers

**Test Command**: `cargo test -p edgequake-api delete`
**Commit**: `refactor(api): Extract delete handlers to documents/delete.rs`

#### Step 9: Extract File Upload Handler (~900 lines)

**Target**: `files.rs`

Move from `documents.rs`:

- `upload_file` function (lines 1786-2189)
- File processing helpers

**Test Command**: `cargo test -p edgequake-api upload_file`
**Commit**: `refactor(api): Extract file upload handler to documents/files.rs`

#### Step 10: Extract Batch Upload Handler (~600 lines)

**Target**: `batch.rs`

Move from `documents.rs`:

- Batch upload functions (lines 2190-3315)
- Batch processing helpers

**Test Command**: `cargo test -p edgequake-api batch`
**Commit**: `refactor(api): Extract batch upload handler to documents/batch.rs`

#### Step 11: Move Tests

Each module gets its own test section at the bottom.

**Test Command**: `cargo test --workspace --lib`
**Commit**: `refactor(api): Distribute tests to respective document modules`

#### Step 12: Remove Original File

```bash
rm edgequake/crates/edgequake-api/src/handlers/documents.rs
```

**Test Command**: `cargo test --workspace`
**Commit**: `refactor(api): Complete documents.rs modularization (3573→7 modules)`

### Success Criteria

✅ All 188 API tests pass  
✅ No new clippy warnings  
✅ Code compiles without errors  
✅ Each module <700 lines  
✅ Public API unchanged  
✅ Documentation updated

### Rollback Plan

If any step fails:

1. Revert last commit: `git revert HEAD`
2. Run tests to verify stability
3. Analyze failure root cause
4. Adjust strategy and retry

Next: Act phase to execute the plan.
