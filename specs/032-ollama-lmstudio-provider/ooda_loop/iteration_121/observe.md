# OODA Iteration 121: Observe

## Date: 2026-01-14

## Mission Reference

Spec requirements 22-25 from [032-ollama-lmstudio-provider.md](../../032-ollama-lmstudio-provider.md):

1. **Item 22**: Default provider/model from models.toml config file
2. **Item 23**: Document upload uses workspace LLM for ingestion
3. **Item 24**: Query uses workspace embedding provider
4. **Item 25**: Rebuild embeddings analysis (re-embed vs recreate)

## Observations

### 1. Default Configuration Mismatch

**models.toml** ([edgequake/models.toml](../../../../edgequake/models.toml#L28-L32)):

```toml
[defaults]
llm_provider = "ollama"
llm_model = "gemma3:12b"
embedding_provider = "ollama"
embedding_model = "embeddinggemma"
```

**Code constants** ([multitenancy.rs](../../../../edgequake/crates/edgequake-core/src/types/multitenancy.rs#L318-L330)):

```rust
pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";  // MISMATCH!
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "openai";  // MISMATCH!
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 1536;  // OpenAI dimension
```

**Issue**: Code defaults use OpenAI for embedding while models.toml uses Ollama.

### 2. Document Upload - Workspace LLM Usage

**Handler**: [documents.rs#L241](../../../../edgequake/crates/edgequake-api/src/handlers/documents.rs#L241)

```rust
let workspace_pipeline = state
    .create_workspace_pipeline(&workspace_id_for_storage)
    .await;
let result = workspace_pipeline
    .process(&document_id, &request.content)
    .await?;
```

**create_workspace_pipeline** ([state.rs#L903-L970](../../../../edgequake/crates/edgequake-api/src/state.rs#L903-L970)):

- ✅ Properly fetches workspace configuration
- ✅ Creates workspace-specific LLM and embedding providers
- ✅ Falls back to global pipeline if workspace not found

**Status**: Document upload correctly uses workspace LLM for ingestion.

### 3. Query - Workspace Embedding Usage

**Handler**: [query.rs#L160-L185](../../../../edgequake/crates/edgequake-api/src/handlers/query.rs#L160-L185)

```rust
match get_workspace_embedding_provider(&state, workspace_id).await {
    Ok(Some(embedding_provider)) => {
        state.sota_engine
            .query_with_embedding_provider(engine_request, embedding_provider)
            .await
    }
    // ...
}
```

**Status**: Query correctly uses workspace embedding provider.

### 4. Rebuild Embeddings Analysis

**Current implementation** ([workspaces.rs#L814-L973](../../../../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L814-L973)):

1. Gets workspace configuration
2. Clears vector storage for workspace only
3. Returns "vectors_cleared" status
4. Does NOT actually re-embed documents

**Gap**: Rebuild clears vectors but doesn't trigger re-embedding of documents.

### 5. Error from Screenshot

The error "Cannot use provider 'openai': Configuration error: OPENAI_API_KEY is empty or invalid" occurs when:

- User explicitly selects OpenAI in the query UI
- But OPENAI_API_KEY is not set

This is **correct behavior** - the error is informative and expected.

## Key Issues Identified

| Issue                                                   | Severity | Fix Required                     |
| ------------------------------------------------------- | -------- | -------------------------------- |
| Default constants don't match models.toml               | High     | Sync constants with models.toml  |
| Rebuild embeddings doesn't re-embed                     | High     | Add document reprocessing logic  |
| Missing link between models.toml and workspace defaults | Medium   | Load from models.toml at startup |

## Next Steps

1. **Orient**: Analyze the best approach to sync defaults
2. **Decide**: Choose between env vars, models.toml, or hybrid approach
3. **Act**: Implement fixes and document rationale
