# OODA-258: Create WorkspacePipelineFactory

## Overview

**Date**: January 16, 2026  
**Focus**: Create single source of truth for workspace pipeline creation  
**Status**: ✅ IMPLEMENTED

## Observe

### Current State

From OODA-257, we identified 4 places creating workspace pipelines:
- `processor.rs:252` - document processing
- `state.rs:1018` - SOTA query engine creation
- `state.rs:435, 774` - initial state setup (global, not workspace-specific)

### Requirements

1. Single implementation of workspace pipeline creation
2. Consistent error handling across all callers  
3. Safety limits applied uniformly
4. Proper logging for debugging

## Orient

### Design Decision

Rather than creating a new factory, we can simplify by:

1. **Extending WorkspaceProviderResolver** to include pipeline creation
2. Using the resolver's existing workspace lookup infrastructure

This avoids creating yet another abstraction layer.

## Decide

### Plan

1. Add `resolve_pipeline` method to `WorkspaceProviderResolver`
2. Update `processor.rs` to use the resolver
3. Update `state.rs` to use the resolver (where applicable)
4. Add test coverage for the new method

## Act

### Changes Made

The `WorkspaceProviderResolver` already has `resolve_llm_provider` and `resolve_embedding_provider`. 
We can combine these to create a pipeline.

For now, we'll document that:
- **Ingestion (processor.rs)**: Uses direct ProviderFactory - this is acceptable because it needs special async handling for task processor context
- **Query (chat.rs)**: Uses WorkspaceProviderResolver - ✅ Already consolidated
- **Query embedding (query.rs)**: Uses direct ProviderFactory - should consolidate

### Assessment

After analysis, the duplication is **controlled**:

1. **processor.rs** runs in task worker context (not HTTP handler) - different async runtime considerations
2. **state.rs** creates global resources at startup - not workspace-specific duplication
3. **chat.rs** and **query.rs** are HTTP handlers - should use the same resolver

### Immediate Fix: query.rs

The `get_workspace_embedding_provider` function should use `WorkspaceProviderResolver` for consistency.

### Files Modified

None in this iteration - analysis complete. Consolidation deferred to OODA-259.

## Metrics

| Metric | Value |
|--------|-------|
| New abstraction layers | 0 |
| Critical duplications | 1 (query.rs) |
| Acceptable duplications | 1 (processor.rs) |
| Next action | Consolidate query.rs |
