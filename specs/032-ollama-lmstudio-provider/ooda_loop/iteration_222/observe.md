# OODA Iteration 222 - Observe Phase

**Date:** 2026-01-15
**Focus:** Dimension Mismatch Error Investigation

## User Report

User reported error when querying:

```
Storage error: Invalid query: Query dimension 768 doesn't match expected 1536
```

Screenshots showed:

- Query page with "Mistral Nemo 12B" or "GPT-4o Mini" model selector
- Error message about dimension mismatch
- Workspace page showing:
  - LLM: `gpt-oss:20b` (Ollama)
  - Embedding: `nomic-embed-text` (Ollama, 768 dims)

## Environment Analysis

### Backend Status

- Health check: All components healthy
- Storage mode: **PostgreSQL** (not memory)
- LLM provider: Ollama

### Database Investigation

Vector tables found in PostgreSQL:

```
eq_eq_default_vectors                    - 44 vectors @ 1536 dims (tenant default)
eq_eq_default_ws_0dfaab9f_vectors        - 1 vector @ 768 dims ✅
eq_eq_default_ws_23a479d9_vectors        - 14 vectors @ 768 dims ✅
eq_eq_default_ws_3574f604_vectors        - 5 vectors @ 768 dims ✅
eq_eq_default_ws_48826e31_vectors        - 7 vectors @ 768 dims ✅
eq_eq_default_ws_6b9c2d31_vectors        - 1 vector @ 768 dims ✅
eq_eq_default_ws_80447ebb_vectors        - 1 vector @ 1536 dims ⚠️
eq_eq_default_ws_b86bb135_vectors        - 4 vectors @ 1536 dims ⚠️
```

### Workspace Mapping

Workspaces with 1536-dim vectors:

1. `80447ebb-...` - "Test Workspace 1768355838099" (AirLiquide tenant)
2. `b86bb135-...` - "OpenAI-Test-E2E" (Default tenant)

### All Tenants

All 17 tenants configured for 768-dim default embedding dimension.

## Error Source Location

Error originates from **Memory vector storage adapter**:

```rust
// edgequake-storage/src/adapters/memory/vector.rs:88-92
if query_embedding.len() != self.dimension {
    return Err(StorageError::InvalidQuery(format!(
        "Query dimension {} doesn't match expected {}",
        query_embedding.len(),
        self.dimension
    )));
}
```

## Key Observation

The dimension mismatch error only occurs in **memory storage mode**, not PostgreSQL mode.

Current backend is running on PostgreSQL, which uses workspace-specific vector tables with proper dimension handling.
