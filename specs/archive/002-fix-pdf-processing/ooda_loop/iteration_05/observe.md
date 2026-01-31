# OODA-05 Observe Phase

## Date: 2026-01-31
## Issue: Embedding Dimension Mismatch in Entity Storage

## Evidence

### Backend Logs
```
2026-01-31T16:07:35.119864Z  WARN edgequake_api::processor: Failed to store entity embedding entity:Agentic Platform: Invalid query: Embedding dimension mismatch: expected 1536, got 768
2026-01-31T16:07:35.119897Z  WARN edgequake_api::processor: Failed to store entity embedding entity:OHcSHaGHfDFHsIHsIHhAHDHen: Invalid query: Embedding dimension mismatch: expected 1536, got 768
```

### Backend Initialization
```
2026-01-31T16:03:32.085099Z  INFO edgequake_api::state: Vector storage validated successfully 
provider="openai" dimension=1536 storage_type="postgres" namespace="default" recreated=false
```

## Root Cause

### Code Analysis

**processor.rs Line 1038:**
```rust
// Entity embeddings use global vector_storage (wrong!)
if let Err(e) = self
    .vector_storage
    .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
```

**processor.rs Line 823:**
```rust
// Chunk embeddings use workspace_vector_storage (correct!)
if workspace_vector_storage
    .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
```

### The Bug
Entity embeddings are stored in the **default global vector storage** (`self.vector_storage`) which is initialized with OpenAI dimensions (1536), but chunks are correctly stored in the **workspace-specific vector storage** (`workspace_vector_storage`) which uses the workspace's embedding provider dimension.

When a workspace uses Ollama (768 dimensions), chunks store successfully but entities fail because they try to store 768-dim vectors in a 1536-dim table.

## Impact

1. **Entity embeddings lost**: All entity embeddings for Ollama-based workspaces fail to store
2. **query_local broken**: Entity-based vector search cannot find entities without embeddings
3. **Inconsistent data**: Chunks are stored correctly but entities are not

## Affected Documents
Any document processed with a non-OpenAI embedding provider (Ollama, etc.) that has embedding dimension ≠ 1536.

## Fix Required
Change entity embedding storage to use `workspace_vector_storage` instead of `self.vector_storage`.
