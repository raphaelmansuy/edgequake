# OODA Iteration 123: Observe

## Date: 2026-01-14

## Mission Checkpoint

Focus on SPEC-032 Items 24-25:

- Item 24: Fix rebuild embeddings - document processing verification
- Item 25: Chunk size vs embedding model compatibility (CRITICAL INVARIANT)

## Observations

### 1. Current Rebuild Embeddings Flow

From OODA 121, the rebuild_embeddings handler:

1. Gets workspace configuration
2. Updates embedding config if changed (model, provider, dimension)
3. Clears vector storage
4. Queues documents for reprocessing

Let me verify the implementation is complete.

### 2. Chunk-Embedding Compatibility Gap

**Problem**: Different embedding models have different max input token limits:

| Model                         | Max Input Tokens | ~Max Characters |
| ----------------------------- | ---------------- | --------------- |
| OpenAI text-embedding-3-small | 8191             | ~32,000         |
| Ollama embeddinggemma         | ~2048            | ~8,000          |
| Ollama nomic-embed-text       | 8192             | ~32,000         |

**Risk**: If we switch from OpenAI (32K chars) to Ollama embeddinggemma (8K chars), existing chunks may exceed the new model's limit.

### 3. models.toml Analysis

Need to verify max_input_tokens is defined for all embedding models.

### 4. Files to Review

| File                                       | Purpose                                     |
| ------------------------------------------ | ------------------------------------------- |
| `edgequake/models.toml`                    | Check max_input_tokens for embedding models |
| `edgequake-api/src/handlers/workspaces.rs` | Verify rebuild_embeddings flow              |
| `edgequake-pipeline/src/chunker.rs`        | Understand chunking logic                   |

## Next Steps

1. Review models.toml for max_input_tokens
2. Review rebuild_embeddings implementation
3. Review chunker to understand current limits
4. Design chunk-model compatibility check
