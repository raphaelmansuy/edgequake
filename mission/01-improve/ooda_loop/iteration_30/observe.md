# OODA-30 — Observe

## Target: Query Engine & Strategy Pure Functions

### Files Analyzed

1. **`crates/edgequake-query/src/engine.rs`** (753 lines, 4 tests)
   - `QueryEngineConfig`: Default impl with Hybrid mode, 20 chunks, 60 entities, 30000 tokens
   - `QueryRequest`: 14 builder methods, all pure
   - `with_llm_full_id`: Parses "provider/model" format — edge cases: no slash, empty string, multiple slashes
   - Existing tests: basic builder, system_prompt, serde round-trip, config default (minimal)

2. **`crates/edgequake-query/src/strategies/config.rs`** (66 lines, 0 tests)
   - `StrategyConfig`: Default with max_chunks=20, max_entities=60, weights 0.5/0.5
   - No edge case tests for defaults or field values

3. **Strategy files without WHY comments** (6 files):
   - `naive.rs`, `hybrid.rs`, `local.rs`, `global.rs`, `mod.rs`, `mix.rs`
   - All are async strategies needing storage — not testable without mocks
   - But WHY comments for architecture decisions are still valuable

### Test Gaps

| Function | Tests | Gap |
|----------|-------|-----|
| StrategyConfig::default() | 0 | All 7 fields untested |
| QueryEngineConfig::default() | 1 (partial) | Only 3 of 9 fields checked |
| QueryRequest::new() | 1 (partial) | Default fields not fully asserted |
| with_llm_full_id() | 0 | Provider/model parsing untested |
| with_tenant_id/workspace_id | 0 | Param insertion untested |
| tenant_id()/workspace_id() | 0 | Param extraction untested |
| with_rerank/with_rerank_top_k | 0 | Rerank overrides untested |
| with_allowed_document_ids | 0 | Document filter untested |
| with_conversation_history | 0 | History builder untested |
