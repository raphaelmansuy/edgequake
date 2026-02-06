# OODA-12 Observe: Data Model Audit

## Mission Re-read
Re-read `specs/001-e2e-upload-pdf.md` — Requirement #5: "Data Model Solidity: Review and ensure document/task data structures are well-designed"

## Data Model Inventory

### DTO Files Audited
| File | Structs | Lines | Tests |
|------|---------|-------|-------|
| documents_types.rs | 18 structs | 1191 | 13 unit tests |
| tasks_types.rs | 6 structs | 371 | 8 unit tests |
| query_types.rs | 5 structs | 357 | unit tests |
| workspaces_types.rs | 12+ structs | 785 | unit tests |
| costs_types.rs | 7 structs | ~200 | unit tests |

### Key Types
- `UploadDocumentRequest`: 8 fields (content, title, metadata, async, track_id, gleaning opts)
- `UploadDocumentResponse`: 9 fields (doc_id, status, task_id, track_id, duplicate_of, counts, cost)
- `DocumentSummary`: 26 fields (id, title, content info, status, pipeline stage, cost, model info)
- `DocumentDetailResponse`: 24 fields (id, content, hash, status, lineage, metadata, pdf_id)
- `TaskResponse`: 15 fields (track_id, tenant/workspace, type, status, timestamps, error, progress)
- `QueryResponse`: 5 fields (answer, mode, sources, stats, conversation_id, reranked)

## Findings

### Strengths
1. Well-separated DTOs per domain (SRP)
2. Proper `skip_serializing_if = "Option::is_none"` usage
3. utoipa ToSchema for OpenAPI docs
4. SPEC annotations on fields
5. Comprehensive unit tests in each type file

### Issues Found
1. **DocumentSummary** has 26 fields — approaching "god struct" territory
2. Cost fields duplicated across DocumentSummary, DocumentCostInfo, DocumentLineage
3. `status` is `Option<String>` in summary but `String` in detail — minor inconsistency
4. No E2E tests validating actual response structure from API endpoints

### Validation Layer
- `validation.rs` already validates content (empty, size limit)
- File validation in `file_validation.rs`
- Path validation in `path_validation.rs`
