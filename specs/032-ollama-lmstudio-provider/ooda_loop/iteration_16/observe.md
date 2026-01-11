# OODA Loop Iteration 16 - Observe Phase

**Date:** 2026-01-11  
**Focus:** Wire LLM Provider Override into Query Engine  
**Status:** ✅ COMPLETE

## Observation

After iteration 15 added `llm_provider` and `llm_model` fields to `QueryRequest`, we need to wire them through to the actual LLM generation step.

### Current State

1. **QueryRequest** has `llm_provider` and `llm_model` fields (from OODA-15)
2. **chat.rs** passes these from `request.provider` (from OODA-15)
3. **SOTAQueryEngine** uses `self.llm_provider` for generation (fixed, not per-request)

### Gap Identified

The engine ignores the request's LLM override. When a user selects "ollama/gemma3:12b" in the UI, the request carries this info but the engine still uses its default LLM.

### Key Files

| File | Role |
|------|------|
| `sota_engine.rs` | Query execution with LLM generation |
| `chat.rs` | API handler that passes provider to engine |
| `factory.rs` | Creates LLM providers by name |
