# OODA-11: Orient - Health API Enhancement Analysis

## First Principles Analysis

### Why Health API Needs Full Configuration Visibility

1. **Debugging Production Issues**: When ingestion fails, operators need to verify:
   - Which LLM model is being used? (affects entity extraction quality)
   - Which embedding model is used? (affects semantic search quality)
   - What's the embedding dimension? (must match vector storage schema)
   - Is PDF storage enabled? (affects document processing)

2. **Configuration Drift Detection**: Different environments may have incorrect settings
   - Dev might use `mock` provider accidentally
   - Production might use wrong model version
   - Embedding dimension mismatch causes silent failures

3. **Operational Monitoring**: Health checks are often the first thing operators check

## Risk Assessment

| Approach                              | Risk                                | Benefit                             |
| ------------------------------------- | ----------------------------------- | ----------------------------------- |
| Add fields to existing HealthResponse | Low - backwards compatible          | Full visibility, easy to understand |
| Create separate `/config` endpoint    | Medium - extra endpoint to maintain | Cleaner separation of concerns      |
| Add nested `providers` object         | Low - clean structure               | Groups related fields logically     |

## Recommended Solution

**Add nested `providers` object to HealthResponse**

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgresql",
  "workspace_id": "default",
  "components": {...},
  "providers": {
    "llm": {
      "name": "ollama",
      "model": "gemma3:latest"
    },
    "embedding": {
      "name": "ollama",
      "model": "nomic-embed-text",
      "dimension": 768
    }
  },
  "pdf_storage_enabled": true,
  "schema": {...}
}
```

## Rationale

1. **Backwards Compatibility**: Keep `llm_provider_name` for existing clients
2. **Logical Grouping**: Provider details grouped under `providers` object
3. **Minimal Changes**: ~50 lines of code changes
4. **Type Safety**: New structs with proper serialization

## Design

```text
HealthResponse (enhanced)
├── status: String
├── version: String
├── storage_mode: String
├── workspace_id: String
├── components: ComponentHealth
├── llm_provider_name: Option<String>  (kept for backward compat)
├── providers: Option<ProvidersHealth> (NEW)
│   ├── llm: LlmProviderHealth
│   │   ├── name: String
│   │   └── model: String
│   └── embedding: EmbeddingProviderHealth
│       ├── name: String
│       ├── model: String
│       └── dimension: usize
├── pdf_storage_enabled: Option<bool>  (NEW)
└── schema: Option<SchemaHealth>
```

## Implementation Plan

1. Add `ProvidersHealth`, `LlmProviderHealth`, `EmbeddingProviderHealth` structs to `health_types.rs`
2. Add `providers` and `pdf_storage_enabled` fields to `HealthResponse`
3. Update `health_check` handler to populate new fields
4. Add unit tests for new fields
5. Verify via curl that response includes all fields
