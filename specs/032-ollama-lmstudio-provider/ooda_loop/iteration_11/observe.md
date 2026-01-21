# Iteration 11: Observe

**Date:** 2025-01-30
**Focus:** Ingestion Pipeline Workspace LLM Integration

## What We Observed

### 1. Current Pipeline Architecture

The document ingestion pipeline has a single global LLM provider:

```
AppState
├── pipeline: Arc<Pipeline>          ← Global, configured at startup
│   ├── extractor: Arc<LLMExtractor>
│   └── embedding_provider: Arc<dyn EmbeddingProvider>
```

### 2. Document Upload Flow

```
POST /api/v1/documents
    → upload_document handler
    → state.pipeline.process(doc_id, content)  ← Uses global pipeline
    → Store chunks, entities, relationships
```

### 3. Missing Infrastructure

| Component                  | Status     | Issue                                     |
| -------------------------- | ---------- | ----------------------------------------- |
| `create_llm_provider()`    | ❌ MISSING | Only `create_embedding_provider()` exists |
| Workspace pipeline         | ❌ MISSING | No per-workspace pipeline creation        |
| Dynamic provider selection | ❌ MISSING | Pipeline hardcoded at startup             |

### 4. Provider Factory Analysis

`edgequake-llm/src/factory.rs`:

- `create_embedding_provider(provider, model, dimension)` ✅ EXISTS
- `create_llm_provider(provider, model)` ❌ MISSING

### 5. Workspace LLM Config

Already stored in database (iteration 09):

- `llm_model: String`
- `llm_provider: String`
- `llm_full_id()` helper method
