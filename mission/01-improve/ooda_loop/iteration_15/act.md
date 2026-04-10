# OODA-15: Act — Production unwrap() Audit

## Commit

`c0af5d98` on `feat/edgequake-v0.9.9`

## Changes

| File                                                          | Line(s)  | Change                                                                       |
| ------------------------------------------------------------- | -------- | ---------------------------------------------------------------------------- |
| `edgequake-pipeline/src/merger/entity.rs`                     | 55,68    | `from_f64().unwrap()` → `unwrap_or(Number::from(0))` — NaN/Inf safe          |
| `edgequake-pipeline/src/merger/relationship.rs`               | 55,68    | Same NaN/Inf safety                                                          |
| `edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs` | 143,200  | `parse().unwrap()` → `unwrap_or(Pending)` — DB migration safe                |
| `edgequake-storage/src/adapters/postgres/pdf_list_query.rs`   | 92       | Same DB migration safety + added PdfProcessingStatus import                  |
| `edgequake-query/src/sota_engine/reranking.rs`                | ~22      | `unwrap()` → `expect("reranker checked above")` with WHY                     |
| `edgequake-storage/src/community.rs`                          | 2 sites  | `unwrap()` → `expect()` with invariant WHY                                   |
| `edgequake-api/src/middleware.rs`                             | 75,83,93 | `unwrap()` → `expect("integer string is valid header")`                      |
| `edgequake-pipeline/src/prompts/parser/json_parser.rs`        | 6 sites  | `Regex::new().unwrap()` → `expect("static regex")`                           |
| `edgequake-core/src/conversation_service/in_memory.rs`        | 21 sites | `.read/write().unwrap()` → `.unwrap_or_else(\|e\| e.into_inner())` + WHY doc |
| `edgequake-tasks/src/memory.rs`                               | 7 sites  | Same RwLock poison recovery                                                  |
| `edgequake-query/src/keywords/mock_extractor.rs`              | 1 site   | Same RwLock poison recovery                                                  |
| `edgequake-api/src/handlers/documents/recovery/reprocess.rs`  | L12      | Added missing `ApiError` import                                              |
| `edgequake-api/src/handlers/pdf_upload/operations.rs`         | L6       | Added missing `ResultExt` import                                             |

## Evidence

- **1,147 tests pass** (534+5+34+137+179+92+15+79+72), 0 failures
- **0 clippy warnings**
- Zero remaining `.read().unwrap()` / `.write().unwrap()` in production code
