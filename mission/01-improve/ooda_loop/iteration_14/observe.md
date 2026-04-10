# OODA-14: Observe

## Remaining DRY Violations

Grep for `map_err(|e| ApiError::Internal(format!(` across handler files found 49 remaining sites in 12 files:

| File                                           | Count |
| ---------------------------------------------- | ----- |
| auth/api_keys.rs                               | 7     |
| auth/mod.rs                                    | 3     |
| auth/session.rs                                | 12    |
| auth/user_management.rs                        | 5     |
| chat/completion.rs                             | 11    |
| chat/streaming.rs                              | 4     |
| workspaces/bulk_ops/mod.rs                     | 1     |
| workspaces/bulk_ops/rebuild_embeddings.rs      | 1     |
| workspaces/bulk_ops/rebuild_knowledge_graph.rs | 2     |
| workspaces/stats.rs                            | 4     |
| query/document_filter_resolver.rs              | 2     |

Some files also had UUID parse patterns convertible to `parse_uuid()` (session.rs: 2 sites).

`document_filter_resolver.rs` used fully-qualified `crate::error::ApiError` paths instead of imports.
