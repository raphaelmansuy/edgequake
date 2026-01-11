# OODA Iteration 07: Orient

## Date: 2025-01-27

## Analysis

### Problem Space

The workspace-level embedding configuration is critical for SPEC-032 because:

1. **Multi-provider Support**: Different workspaces may need different embedding providers
2. **Cost Optimization**: OpenAI costs money, Ollama/LM Studio are local and free
3. **Dimension Compatibility**: Vectors must have consistent dimensions within a workspace
4. **Migration Safety**: Changing embedding model requires vector database rebuild

### Architecture Decisions

1. **Module-Level Constants**
   - `DEFAULT_EMBEDDING_MODEL = "text-embedding-3-small"`
   - `DEFAULT_EMBEDDING_PROVIDER = "openai"`
   - `DEFAULT_EMBEDDING_DIMENSION = 1536`

2. **Auto-Detection Logic**
   - Model name patterns → provider detection
   - Model name patterns → dimension detection
   - Environment variable overrides

3. **Storage Strategy**
   - Embed config in metadata JSONB (backward compatible)
   - Will add dedicated columns in future migration

### Design Patterns

1. **Builder Pattern** for CreateWorkspaceRequest
   - `.new(name)` creates minimal request
   - `.with_embedding_model(model)` configures embedding

2. **Helper Function** for response conversion
   - Centralized `workspace_to_response(&Workspace)` 
   - Ensures embedding fields always included

3. **Environment-Based Defaults**
   - `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`
   - `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`
   - `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION`

## Impact Assessment

| Component | Impact | Risk |
|-----------|--------|------|
| Workspace struct | High | Low (additive) |
| API DTOs | Medium | Low (optional fields) |
| Services | Medium | Low (defaults applied) |
| Tests | High | Medium (many updates) |
| Database | Low | None (metadata JSONB) |
