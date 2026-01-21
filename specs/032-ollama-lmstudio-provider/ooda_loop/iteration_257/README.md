# OODA-257: Pipeline Creation Duplication Audit

## Overview

**Date**: January 16, 2026  
**Focus**: Identifying and documenting workspace pipeline creation duplication  
**Status**: ✅ ANALYSIS COMPLETE, ⏳ REFACTORING PENDING

## Observe

### Pipeline Creation Patterns Found

Searched for `Pipeline::default_pipeline()` usage in the API crate:

| Location                                                                        | Purpose                         | Type               |
| ------------------------------------------------------------------------------- | ------------------------------- | ------------------ |
| [state.rs#L435](../../edgequake/crates/edgequake-api/src/state.rs#L435)         | Memory state initialization     | Global             |
| [state.rs#L774](../../edgequake/crates/edgequake-api/src/state.rs#L774)         | PostgreSQL state initialization | Global             |
| [state.rs#L1018](../../edgequake/crates/edgequake-api/src/state.rs#L1018)       | `create_workspace_sota_engine`  | Workspace-specific |
| [processor.rs#L252](../../edgequake/crates/edgequake-api/src/processor.rs#L252) | `get_workspace_pipeline`        | Workspace-specific |

### Duplication Risk

**Lines 1018 (state.rs) and 252 (processor.rs) are SEMANTIC DUPLICATES**:

```rust
// state.rs:1018-1020
Pipeline::default_pipeline()
    .with_extractor(extractor)
    .with_embedding_provider(embedding),

// processor.rs:252-254
Pipeline::default_pipeline()
    .with_extractor(extractor)
    .with_embedding_provider(Arc::clone(embedding)),
```

Both:

1. Look up workspace by ID
2. Parse UUID
3. Create LLM provider via `ProviderFactory::create_safe_llm_provider`
4. Create embedding provider via `ProviderFactory::create_safe_embedding_provider`
5. Build pipeline with extractor and embedding

### Failure Surface

If someone updates the provider creation logic in one place but not the other:

- **Scenario A**: Add retry logic to processor.rs but not state.rs → inconsistent reliability
- **Scenario B**: Add new provider type to state.rs but not processor.rs → feature gap
- **Scenario C**: Change safety limits in one place → security inconsistency

**Reliability theory**: System reliability = Π(component reliability). Duplicated code doubles failure surface.

## Orient

### Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Request Flow                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐      ┌───────────────────┐                    │
│  │   chat.rs    │      │ processor.rs      │                    │
│  │              │      │                   │                    │
│  │ Uses:        │      │ Uses:             │                    │
│  │ WorkspacePro │      │ ProviderFactory   │  ← INCONSISTENT   │
│  │ viderResolve │      │ directly          │                    │
│  │ r (OODA-227) │      │                   │                    │
│  └──────────────┘      └───────────────────┘                    │
│         │                      │                                 │
│         ▼                      ▼                                 │
│  ┌──────────────────────────────────────────┐                   │
│  │          ProviderFactory                  │                   │
│  │  (edgequake-llm crate)                   │                   │
│  └──────────────────────────────────────────┘                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Single Source of Truth Requirement

According to the mission statement OODA-226+:

> **Axiom of Single Truth**: Every piece of business logic MUST exist in exactly one place

The `WorkspaceProviderResolver` was created in OODA-226 to be the single source of truth for provider resolution. However:

1. ✅ `chat.rs` uses `WorkspaceProviderResolver` (OODA-227)
2. ❌ `processor.rs` still uses `ProviderFactory` directly
3. ❌ `state.rs` uses `ProviderFactory` directly in `create_workspace_sota_engine`
4. ❌ `query.rs` uses `ProviderFactory` directly in `get_workspace_embedding_provider`

## Decide

### Proposed Solution: PipelineFactory

Create a `PipelineFactory` that encapsulates workspace-specific pipeline creation:

```rust
// edgequake-api/src/providers/pipeline_factory.rs (NEW)
pub struct WorkspacePipelineFactory {
    workspace_service: Arc<dyn WorkspaceService>,
}

impl WorkspacePipelineFactory {
    /// Create a workspace-specific pipeline with proper providers.
    ///
    /// Uses WorkspaceProviderResolver internally for consistent resolution.
    pub async fn create_workspace_pipeline(
        &self,
        workspace_id: &str,
    ) -> Result<Arc<Pipeline>, ProviderResolutionError> {
        // Single implementation here
    }
}
```

### Consolidation Plan

| File         | Current                | After                     |
| ------------ | ---------------------- | ------------------------- |
| processor.rs | Direct ProviderFactory | WorkspacePipelineFactory  |
| state.rs     | Direct ProviderFactory | WorkspacePipelineFactory  |
| query.rs     | Direct ProviderFactory | WorkspaceProviderResolver |

### Priority

**HIGH** - This duplication has direct reliability impact on:

- Document ingestion (processor.rs)
- Query execution (state.rs, query.rs)
- Chat completions (chat.rs - already fixed)

## Act

### Immediate Action

1. Created this documentation (OODA-257)
2. Identified 4 duplication points
3. Proposed PipelineFactory solution

### Deferred to OODA-258

1. Create `WorkspacePipelineFactory` module
2. Refactor `processor.rs` to use it
3. Refactor `state.rs` to use it
4. Add tests for the factory

## Metrics

| Metric                          | Value     |
| ------------------------------- | --------- |
| Duplication points found        | 4         |
| Using WorkspaceProviderResolver | 1/4 (25%) |
| Direct ProviderFactory usage    | 3/4 (75%) |
| Test coverage risk              | High      |
| Reliability impact              | High      |
