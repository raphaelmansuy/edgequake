# P0 — WorkspacePipelineFactory (API-DRY-001, API-SOLID-L-001)

**Status:** ✅ Proven  
**Date:** 2026-06-04

## Claim

Single source of truth for workspace-scoped ingestion pipelines with explicit `Strict | LenientGlobal` fallback policies.

## Evidence

```bash
cargo test -p edgequake-api --test spec017_api_contract spec017_single_workspace_pipeline_factory
cargo test -p edgequake-api --test e2e_workspace_pipeline_integration
```

| Check | Result |
|-------|--------|
| `WorkspacePipelineFactory` in `workspace_pipeline_factory.rs` | ✅ |
| `AppState::create_workspace_pipeline` delegates to factory | ✅ |
| `DocumentTaskProcessor::get_workspace_pipeline*` delegates to factory | ✅ |
| No inline `create_safe_llm_provider` in state/processor | ✅ |
| Strict vs Lenient unit tests | ✅ 4/4 in factory module |
