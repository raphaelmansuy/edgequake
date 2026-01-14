# OODA Iteration 121: Orient

## Date: 2026-01-14

## Analysis of Default Configuration

### Current Configuration Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          CURRENT CONFIGURATION FLOW                          │
└─────────────────────────────────────────────────────────────────────────────┘

Environment Variables (optional)
         ↓
         ├── EDGEQUAKE_DEFAULT_LLM_MODEL
         ├── EDGEQUAKE_DEFAULT_LLM_PROVIDER
         ├── EDGEQUAKE_DEFAULT_EMBEDDING_MODEL
         ├── EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER
         └── EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION
         ↓
Code Constants (fallback)           models.toml (IGNORED!)
         ↓                                  ↓
Tenant::new()                        ModelsConfig::load()
         ↓                                  ↓
Workspace::new()                     AppState::models_config
         ↓                                  ↓
 Document Processing              /api/v1/models/* endpoints
```

**Problem**: models.toml is used for API model listing but NOT for workspace defaults!

### Desired Configuration Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          DESIRED CONFIGURATION FLOW                          │
└─────────────────────────────────────────────────────────────────────────────┘

models.toml [defaults] section
         ↓
ModelsConfig::load()
         ↓
AppState::models_config.defaults
         ↓                            Environment Variables
         ├────────────────────────────(override if set)
         ↓
Workspace/Tenant defaults             API Model Listings
         ↓                                  ↓
 Document Processing               /api/v1/models/* endpoints
```

## Analysis of Rebuild Embeddings

### Current Behavior

```rust
// workspaces.rs:rebuild_embeddings
// 1. Clear vectors for workspace
let vectors_cleared = state
    .vector_storage
    .clear_workspace(&workspace_id)
    .await?;

// 2. Return "vectors_cleared" status
// But NO re-embedding happens!
```

### Approaches for Re-embedding

#### Approach A: Full Re-embedding Pipeline

**Steps**:

1. Clear existing vectors for workspace
2. Fetch all documents in workspace
3. Re-chunk documents
4. Re-embed chunks with new embedding model
5. Store new embeddings

**Pros**:

- Clean slate, no legacy data
- Consistent with new model dimensions
- Works for any dimension change

**Cons**:

- High compute cost
- Longer downtime
- More complex implementation

#### Approach B: Update Embeddings In-Place

**Steps**:

1. Fetch all existing chunks from storage
2. Re-embed each chunk with new model
3. Update vector storage with new embeddings

**Pros**:

- Lower compute (no re-chunking)
- Faster processing
- Preserves chunk metadata

**Cons**:

- Dimension mismatch requires recreating storage anyway
- Chunks may not be optimal for new model
- May not work if chunk token limits differ

### Recommended Approach: Hybrid

For **embedding model change**:

1. If dimension changes → Full Re-embedding (Approach A)
2. If dimension same → In-Place Update (Approach B)

**Critical Invariant**: Chunk size must be compatible with embedding model's max input tokens.

## Key Decisions Required

| Decision              | Options                       | Recommendation                       |
| --------------------- | ----------------------------- | ------------------------------------ |
| Default source        | Code constants vs models.toml | Use models.toml, sync constants      |
| Re-embedding approach | Full rebuild vs in-place      | Hybrid based on dimension change     |
| Async vs sync rebuild | Background job vs blocking    | Async with progress tracking         |
| Workspace isolation   | Clear all vs workspace-only   | Workspace-only (already implemented) |

## Dependencies

- `edgequake-llm::ModelsConfig` - Already provides `defaults` struct
- `edgequake-core::Workspace` - Needs to accept external defaults
- `edgequake-api::AppState` - Already holds `models_config`
