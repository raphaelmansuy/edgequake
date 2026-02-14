# OODA-32: Act

## Changes Made

### Files Modified (12 resource modules)
- `health.rs` — +3 methods: ready(), live(), metrics()
- `auth.rs` — +1 method: logout()
- `documents.rs` — +5 methods: delete_all(), reprocess(), recover_stuck(), retry_chunks(), failed_chunks()
- `graph.rs` — +3 methods: get_node(), search_labels(), popular_labels()
- `entities.rs` — +1 method: update()
- `relationships.rs` — +2 methods: get(), update()
- `tasks.rs` — +1 method: retry()
- `pipeline.rs` — +1 method: cancel()
- `costs.rs` — +3 methods: pricing(), estimate(), update_budget()
- `tenants.rs` — +1 method: update()
- `folders.rs` — +1 method: update()
- `conversations.rs` — +5 methods: import(), update(), unshare(), bulk_archive(), bulk_move()
- `models.rs` — +4 methods: list_llm(), list_embedding(), get_provider(), get_model()
- `workspaces.rs` — +7 methods: get(), update(), delete(), metrics_history(), rebuild_embeddings(), rebuild_knowledge_graph(), reprocess_documents()

### Test File
- `tests/integration_tests.rs` — +38 new wiremock tests

### Test Results
- **Before**: 166 tests (OODA-31)
- **After**: 204 tests (85 integration + 118 unit + 1 doc)
- **All passing**: `test result: ok. 204 passed; 0 failed`

### Method Count
- **Before**: ~75 public methods
- **After**: ~113 public methods (+38 endpoints)
- **Remaining gaps**: Streaming endpoints only (query/stream, chat/stream, graph/stream, websocket)
