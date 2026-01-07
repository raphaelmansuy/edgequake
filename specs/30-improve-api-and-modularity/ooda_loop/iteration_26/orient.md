# Iteration 26 - Orient

## Analysis

### Sibling File Approach
The sibling file approach (`documents_types.rs`) maintains:
1. **Flat module structure**: Consistent with existing mod.rs pattern
2. **Clear separation**: DTOs isolated from handler logic
3. **Testability**: Unit tests for DTOs in dedicated module
4. **Import flexibility**: `pub use documents_types::*;` from documents.rs

### DTOs to Extract (22 total)
1. **Upload Group**: UploadDocumentRequest, UploadDocumentResponse, DocumentCostInfo
2. **List Group**: ListDocumentsRequest, ListDocumentsResponse, DocumentSummary, StatusCounts
3. **Detail Group**: GetDocumentRequest, DocumentDetailResponse, DocumentLineage
4. **Delete Group**: DeleteDocumentResponse, DeletionImpactResponse
5. **File Upload Group**: FileUploadResponse, BatchUploadResponse, BatchFileResult
6. **Track Group**: TrackStatusResponse
7. **Scan Group**: ScanDirectoryRequest, ScanDirectoryResponse, SkippedFile
8. **Recovery Group**: ReprocessFailedRequest, ReprocessFailedResponse, RecoverStuckRequest, RecoverStuckResponse

### Helper Functions to Extract
- `default_enable_gleaning()`, `default_max_gleaning()`, `default_use_llm_summarization()`
- `default_page()`, `default_page_size()`
- `default_recursive()`, `default_max_files()`, `default_true()`
- `default_max_reprocess()`, `default_stuck_threshold_minutes()`

## Decision Matrix
| Option | Pros | Cons | Decision |
|--------|------|------|----------|
| Sibling file | Maintains flat structure, tested pattern | Slightly more files | ✅ Selected |
| Submodule | Logical grouping | Conflicts with mod.rs exports | ❌ Rejected in iteration 25 |
| Keep inline | No changes | 3,573 line file, poor maintainability | ❌ Rejected |

## Strategic Direction
Create `documents_types.rs` with all DTOs and helper functions, then update `documents.rs` to import from it.
