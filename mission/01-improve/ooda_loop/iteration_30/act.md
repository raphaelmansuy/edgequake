# OODA-30 — Act

## Changes Made

### 1. WHY comment: `strategies/config.rs` (line 10-20)
- Added ASCII diagram explaining vector_weight/graph_weight blend semantics
- Documented that weights are advisory, not required to sum to 1.0

### 2. Expanded `test_query_engine_config_default` (engine.rs:748)
- Now asserts all 9 QueryEngineConfig fields + 3 TruncationConfig sub-fields (was 3 fields)

### 3. New tests in `engine.rs` (12 new tests):
- `test_strategy_config_default_all_fields` — all 7 StrategyConfig fields
- `test_query_request_new_defaults` — all 14 QueryRequest fields verified as None/empty/false
- `test_with_llm_full_id_with_slash` — "ollama/gemma3:12b" parses correctly
- `test_with_llm_full_id_no_slash` — "openai" treated as provider-only
- `test_with_llm_full_id_multiple_slashes` — "a/b/c" → provider="a", model="b/c"
- `test_with_llm_full_id_empty` — empty string → provider="", no model
- `test_tenant_workspace_id_round_trip` — insertion + extraction via params HashMap
- `test_tenant_workspace_id_absent` — None when not set
- `test_rerank_overrides` — enable_rerank + rerank_top_k
- `test_allowed_document_ids` — document filter Vec<String>
- `test_conversation_history_builder` — multi-turn history
- `test_builder_chain_all_methods` — every builder method chained

## Test Evidence

- **edgequake-query**: 159 passed (was ~145, +14)
- **Workspace total**: 1337 passed, 0 failed
- **Clippy**: 0 warnings
