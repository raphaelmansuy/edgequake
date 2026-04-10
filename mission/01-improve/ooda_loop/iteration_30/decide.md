# OODA-30 — Decide

## Plan

1. Add WHY comment to `strategies/config.rs` explaining weight semantics and LightRAG alignment
2. Add comprehensive tests to `engine.rs`:
   - StrategyConfig::default() — all 7 fields
   - QueryEngineConfig::default() — all 9 fields (expand existing test)
   - QueryRequest::new() defaults — verify all None/empty/false fields
   - `with_llm_full_id` — 4 edge cases: "provider/model", "provider-only", "a/b/c" multi-slash, empty
   - `with_tenant_id` + `tenant_id()` round-trip
   - `with_workspace_id` + `workspace_id()` round-trip
   - `with_rerank` + `with_rerank_top_k`
   - `with_allowed_document_ids`
   - `with_conversation_history`
3. Run tests, commit as OODA-30

**Expected: ~15 new tests**
