# OODA-14: Act

## Changes

Commit: `9309aa72` — `OODA-14: Complete DRY migration of remaining 12 handler files to ResultExt`

### Files Modified (16 files, +162/-159 lines)

| File                                                                    | Sites Changed | Pattern                               |
| ----------------------------------------------------------------------- | ------------- | ------------------------------------- |
| handlers/auth/mod.rs:37,148,161,180                                     | 3             | `map_err→internal_err`                |
| handlers/auth/api_keys.rs:59,86,92,180,184,190,196                      | 7             | `map_err→internal_err`                |
| handlers/auth/session.rs:79,104,110,147,173,178,213,216,221,227,281,285 | 10+2          | `map_err→internal_err` + `parse_uuid` |
| handlers/auth/user_management.rs:70,82,117,133,245                      | 5             | `map_err→internal_err`                |
| handlers/chat/completion.rs:94,128,150,169,329,345,354,360,383,400      | 10            | `map_err→internal_err`                |
| handlers/chat/streaming.rs:100,133,154,173                              | 4             | `map_err→internal_err`                |
| handlers/workspaces/bulk_ops/mod.rs:70                                  | 1             | `map_err→internal_err`                |
| handlers/workspaces/bulk_ops/rebuild_embeddings.rs:169                  | 1             | `map_err→internal_err`                |
| handlers/workspaces/bulk_ops/rebuild_knowledge_graph.rs:122,137         | 2             | `map_err→internal_err`                |
| handlers/workspaces/stats.rs:141,169,183,250                            | 4             | `map_err→internal_err`                |
| handlers/query/document_filter_resolver.rs:33,43,57                     | 2+1           | `map_err→internal_err` + import fix   |
| handlers/documents/recovery/reprocess.rs:12                             | 0             | Unused import fix                     |
| handlers/documents/recovery/stuck.rs:11                                 | 0             | Unused import fix                     |
| handlers/pdf_upload/operations.rs:6                                     | 0             | Unused import fix                     |

## Evidence

- `cargo test -p edgequake-api --lib`: 534 passed, 0 failed
- `cargo clippy -p edgequake-api`: 0 warnings
