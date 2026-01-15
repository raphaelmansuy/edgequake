# OODA Iteration 217 - Observe

## Focus: Query HTTP Flow with Workspace Provider Selection

### Objective

Analyze the complete HTTP flow for query execution to understand how workspace-specific embedding providers are created and used.

### Query Handler Flow Analysis

From [query.rs](../../../../edgequake/crates/edgequake-api/src/handlers/query.rs#L96-L220):

```rust
pub async fn execute_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,            // ← Workspace ID from X-Workspace-Id header
    Json(request): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
```

#### Step 1: Extract Workspace Context

```rust
if let Some(ref workspace_id) = tenant_ctx.workspace_id {
    let embedding_result = get_workspace_embedding_provider(&state, workspace_id).await;
    let vector_result = get_workspace_vector_storage(&state, workspace_id).await;
```

#### Step 2: Create Workspace-Specific Provider

From `get_workspace_embedding_provider()` (lines 466-523):

```rust
// Get workspace from service
let workspace = state
    .workspace_service
    .get_workspace(workspace_uuid)
    .await?;

// Create workspace-specific embedding provider
let provider = ProviderFactory::create_embedding_provider(
    &workspace.embedding_provider,   // e.g., "ollama" or "openai"
    &workspace.embedding_model,      // e.g., "nomic-embed-text" or "text-embedding-3-small"
    workspace.embedding_dimension,   // e.g., 768 or 1536
)?;
```

#### Step 3: Execute Query with Workspace Provider

```rust
match (embedding_result, vector_result) {
    (Ok(Some(embedding_provider)), Ok(Some(vector_storage))) => {
        // Full workspace isolation
        state.sota_engine.query_with_workspace_config(
            engine_request, 
            embedding_provider,    // ← Workspace-specific provider
            vector_storage         // ← Workspace-specific storage
        ).await
    }
    (Ok(Some(embedding_provider)), _) => {
        // Workspace-specific embedding only
        state.sota_engine.query_with_embedding_provider(
            engine_request, 
            embedding_provider     // ← Workspace-specific provider
        ).await
    }
```

### Key Discovery: Provider Selection Points

The query flow has THREE provider selection paths:

| Path | Condition | Provider Used |
|------|-----------|---------------|
| **1. Full workspace isolation** | Both embedding + vector storage exist | Workspace-specific embedding + workspace-specific vector storage |
| **2. Embedding-only isolation** | Embedding exists, no vector storage | Workspace-specific embedding + default vector storage |
| **3. Default provider** | No workspace config | Global embedding provider from `state.embedding_provider` |

### Verification Needed

We need to verify:

1. **HTTP test with X-Workspace-Id header** - Does the header correctly extract workspace context?
2. **ProviderFactory.create_embedding_provider** - Does this correctly create Ollama vs OpenAI providers?
3. **Provider name logging** - Can we capture which provider was used for query lineage?

### Next Steps

1. Create E2E test that:
   - Creates workspace with specific embedding config
   - Executes query via HTTP with X-Workspace-Id header
   - Verifies workspace provider was used (via logs or response metadata)

2. Test provider switching:
   - Execute query with Ollama provider
   - Switch workspace to OpenAI provider
   - Execute query again
   - Verify different provider was used
