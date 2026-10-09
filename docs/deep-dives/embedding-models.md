---
title: 'Deep Dive: Embedding Models'
description: "How EdgeQuake selects and runs embedding models."
---

> **Product: v0.23.0** · Contract: OpenAPI · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

# Deep Dive: Embedding Models

> **Understanding Vector Embeddings in EdgeQuake**

This guide covers how embedding models work in EdgeQuake, how to choose the right one, and optimization strategies.

---

## What Are Embeddings?

Embeddings are **dense vector representations** of text that capture semantic meaning. Similar concepts have similar vectors.

```
┌─────────────────────────────────────────────────────────────────┐
│                 EMBEDDING VISUALIZATIO                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Text: "The cat sat on the mat"                                 │
│                    ↓                                            │
│          ┌─────────────────┐                                    │
│          │ Embedding Model │                                    │
│          └────────┬────────┘                                    │
│                   ↓                                             │
│  Vector: [0.23, -0.15, 0.87, 0.42, ..., -0.31]  (1536 dims)     │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Semantic Space (2D projection)                           │   │
│  │                                                          │   │
│  │     cat●                  ●dog                           │   │
│  │       ↖                  ↗                               │   │
│  │  kitten●   ← similar →   ●puppy                          │   │
│  │                                                          │   │
│  │                                                          │   │
│  │         car●          ●truck                             │   │
│  │               ↖    ↗                                     │   │
│  │             vehicle●                                     │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Similar concepts cluster together in vector space              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Embedding in EdgeQuake

EdgeQuake uses embeddings at multiple stages:

```
┌─────────────────────────────────────────────────────────────────┐
│                 EMBEDDING USAGE                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. DOCUMENT PROCESSING                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Document → Chunks → [Embed] → Store in pgvector          │   │
│  │                                                          │   │
│  │ Entities → [Embed] → Store in pgvector                   │   │
│  │                                                          │   │
│  │ Relationships → [Embed] → Store in pgvector              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  2. QUERY PROCESSING                                            │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Query → [Embed] → Vector Search → Top-K results          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  3. ENTITY MATCHING                                             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ New Entity → [Embed] → Similar Entity Search → Merge?    │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Supported Embedding Models

### OpenAI Models

| Model                    | Dimensions | Max Tokens | Cost/1K  | Quality       |
| ------------------------ | ---------- | ---------- | -------- | ------------- |
| `text-embedding-3-small` | 1536       | 8191       | $0.00002 | Good          |
| `text-embedding-3-large` | 3072       | 8191       | $0.00013 | Excellent     |
| `text-embedding-ada-002` | 1536       | 8191       | $0.00010 | Good (legacy) |

**Recommendation**: Use `text-embedding-3-small` for most use cases (best cost/performance).

```bash
# Defaults from .env.example (workspace bootstrap)
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
export EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1536

# Runtime override (takes precedence over DEFAULT_* when set)
# export EDGEQUAKE_EMBEDDING_PROVIDER=openai
# export EDGEQUAKE_EMBEDDING_MODEL=text-embedding-3-small
# export EDGEQUAKE_EMBEDDING_DIMENSION=1536
```

### Ollama Models

| Model                    | Dimensions | Max Tokens | Cost | Quality   |
| ------------------------ | ---------- | ---------- | ---- | --------- |
| `embeddinggemma:latest`  | 768        | 8192       | Free | Good (default for `make dev`) |
| `nomic-embed-text`       | 768        | 8192       | Free | Good      |
| `mxbai-embed-large`      | 1024       | 512        | Free | Very Good |
| `all-minilm`             | 384        | 256        | Free | Moderate  |
| `snowflake-arctic-embed` | 1024       | 512        | Free | Very Good |

### Mistral Models

| Model           | Dimensions | Notes                          |
| --------------- | ---------- | ------------------------------ |
| `mistral-embed` | 1024       | Native Mistral embedding model |

**Recommendation**: Use `embeddinggemma:latest` for local Ollama deployment (`make dev` default). Use `nomic-embed-text` as an alternative with similar 768-dimension vectors.

```bash
ollama pull embeddinggemma:latest
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=ollama
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=embeddinggemma:latest
export EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=768
```

---

## Dimension Tradeoffs

```
┌─────────────────────────────────────────────────────────────────┐
│                 DIMENSION TRADEOFFS                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Lower Dimensions (384-768):                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ✅ Faster similarity search                               
│  │ ✅ Less storage space                                    
│  │ ✅ Lower memory usage                                    
│  │ ❌ Less semantic precision                               
│  │ ❌ May miss subtle distinctions                          
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Higher Dimensions (1536-3072):                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ✅ Better semantic precision                             
│  │ ✅ Captures subtle nuances                               
│  │ ✅ Better for specialized domains                        
│  │ ❌ Slower similarity search                              
│  │ ❌ More storage required                                 
│  │ ❌ Higher memory usage                                   
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Storage Impact (100K embeddings):                              │
│  • 384 dims:  153 MB                                            │
│  • 768 dims:  307 MB                                            │
│  • 1536 dims: 614 MB                                            │
│    • 3072 dims: 1.2 GB                                            │
  │                                                                 │
  │ halfvec mode (EDGEQUAKE_VECTOR_STORAGE=halfvec): ~50% column   │
  │ footprint; required for HNSW when dim ∈ (2000, 4000]. See      │
  │ [Vector Storage](/docs/deep-dives/vector-storage/).             │
  │                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## EmbeddingProvider Trait

EdgeQuake's embedding system is built on a trait abstraction:

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Get the embedding model.
    fn model(&self) -> &str;

    /// Get the dimension of the embeddings.
    fn dimension(&self) -> usize;

    /// Get the maximum number of tokens per input.
    fn max_tokens(&self) -> usize;

    /// Generate embeddings for a batch of texts.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Generate embedding for a single text.
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>>;
}
```

**Why Trait-Based Design**:

- **Testing**: MockProvider returns deterministic embeddings
- **Flexibility**: Swap providers without code changes
- **Cost control**: Route to different providers based on request
- **Resilience**: Fallback providers when primary unavailable

---

## Similarity Metrics

EdgeQuake uses **cosine similarity** by default with pgvector:

```sql
-- Cosine similarity (default, best for text)
SELECT * FROM embeddings
ORDER BY embedding <=> query_embedding
LIMIT 10;

-- Also available:
-- Inner product: <#>
-- L2 distance: <->
```

**Why Cosine Similarity**:

- Normalized vectors (magnitude doesn't matter)
- Works well for text embeddings
- Range: -1 to 1 (1 = identical)
- pgvector optimized for this metric

---

## Embedding Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                 EMBEDDING PIPELINE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Input: "Artificial intelligence is transforming..."            │
│                            ↓                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 1. TOKENIZATION                                            │ │
│  │    Split text into tokens: ["Artificial", "intelligence",  │ │
│  │                             "is", "transforming", ...]     │ │
│  │    Check: tokens < max_tokens (8191 for OpenAI)            │ │
│  └────────────────────────────────────────────────────────────┘ │
│                            ↓                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 2. BATCHING                                                │ │
│  │    Group texts for efficient API calls                     │ │
│  │    OpenAI: up to 2048 texts per batch                      │ │
│  │    Ollama: 1 text per call (no batching)                   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                            ↓                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 3. API CALL                                                │ │
│  │    POST /v1/embeddings                                     │ │
│  │    {"input": texts, "model": "text-embedding-3-small"}     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                            ↓                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 4. NORMALIZATION                                           │ │
│  │    Ensure unit length: ||v|| = 1                           │ │
│  │    (OpenAI returns pre-normalized, Ollama may not)         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                            ↓                                    │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 5. STORAGE                                                 │ │
│  │    INSERT INTO embeddings (id, embedding, ...)             │ │
│  │    VALUES ($1, $2::vector, ...)                            │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Choosing the Right Model

### Decision Matrix

| Requirement     | Recommended Model                    |
| --------------- | ------------------------------------ |
| Lowest cost     | `nomic-embed-text` (Ollama, free)    |
| Best quality    | `text-embedding-3-large` (OpenAI)    |
| Best value      | `text-embedding-3-small` (OpenAI)    |
| Privacy (local) | `nomic-embed-text` (Ollama)          |
| Low latency     | `all-minilm` (Ollama, 384 dims)      |
| High recall     | `text-embedding-3-large` (3072 dims) |

### Domain Considerations

| Domain            | Recommendation                               |
| ----------------- | -------------------------------------------- |
| General knowledge | `text-embedding-3-small`                     |
| Legal/medical     | `text-embedding-3-large` (precision matters) |
| Multi-language    | `text-embedding-3-small` (good multilingual) |
| Code/technical    | `text-embedding-3-small` + domain chunks     |

---

## Configuration

### Per-Workspace Settings

```bash
# Create workspace with specific embedding model
curl -X POST http://localhost:8080/api/v1/tenants/default/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "research",
    "embedding_model": "text-embedding-3-large",
    "embedding_dimension": 3072
  }'
```

### Global Defaults

Environment variables follow `.env.example` naming:

| Variable | Role |
| -------- | ---- |
| `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER` | Workspace bootstrap default |
| `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL` | Model at workspace creation |
| `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION` | Must match model output |
| `EDGEQUAKE_EMBEDDING_PROVIDER` | Runtime override (wins over `DEFAULT_*`) |
| `EDGEQUAKE_EMBEDDING_MODEL` | Runtime model override |
| `EDGEQUAKE_EMBEDDING_DIMENSION` | Runtime dimension override |
| `EDGEQUAKE_VECTOR_STORAGE` | `full` (vector) or `halfvec` — see [Vector Storage](/docs/deep-dives/vector-storage/) |

```bash
export EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai
export EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
export EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1536
```

### models.toml Configuration

```toml
[defaults]
embedding_provider = "openai"
embedding_model = "text-embedding-3-small"

[providers.openai.embedding_models.text-embedding-3-small]
display_name = "Text Embedding 3 Small"
dimensions = 1536
max_tokens = 8191
price_per_1k_input_tokens = 0.00002

[providers.openai.embedding_models.text-embedding-3-large]
display_name = "Text Embedding 3 Large"
dimensions = 3072
max_tokens = 8191
price_per_1k_input_tokens = 0.00013
```

---

## Changing Embedding Models

**Warning**: Changing embedding models requires rebuilding all embeddings.

```bash
# 1. Update workspace settings
curl -X PUT http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID \
  -d '{"embedding_model": "text-embedding-3-large", "embedding_dimension": 3072}'

# 2. Rebuild embeddings (this reprocesses all documents)
curl -X POST http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID/rebuild-embeddings

# 3. Monitor progress
curl http://localhost:8080/api/v1/tasks?status=running
```

**Why Rebuild Is Required**:

- Different models produce different vector spaces
- Vectors from different models are **not comparable**
- Search would return incorrect results with mixed embeddings

---

## Performance Optimization

### Batch Processing

```
┌─────────────────────────────────────────────────────────────────┐
│                 BATCH VS SEQUENTIAL                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Sequential (slow):                                             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Text 1 → API → Wait → Text 2 → API → Wait → ...          │   │
│  │                                                          │   │
│  │ 100 texts × 100ms = 10 seconds                           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Batched (fast):                                                │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ [Text 1, Text 2, ..., Text 100] → API → All embeddings   │   │
│  │                                                          │   │
│  │ 1 API call = 150ms                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Speedup: 67x faster with batching                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Caching

EdgeQuake caches query embeddings in memory:

```rust
// Pseudo-code: Query embedding caching
let cache_key = hash(query_text);
if let Some(embedding) = cache.get(cache_key) {
    return embedding;  // Cache hit: 0ms
}
let embedding = provider.embed_one(query).await?;  // 50ms
cache.insert(cache_key, embedding);
return embedding;
```

### HNSW Index Tuning

```sql
-- Create optimized HNSW index
CREATE INDEX CONCURRENTLY embeddings_hnsw_idx
ON embeddings
USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Tune search quality/speed tradeoff
SET hnsw.ef_search = 100;  -- Higher = better recall, slower
```

| ef_search | Recall | Latency |
| --------- | ------ | ------- |
| 40        | 95%    | 10ms    |
| 100       | 98%    | 20ms    |
| 200       | 99%    | 40ms    |

---

## Cost Analysis

### OpenAI Embedding Costs

| Model                  | Cost/1M tokens | 100K Docs (500 tokens each) |
| ---------------------- | -------------- | --------------------------- |
| text-embedding-3-small | $0.02          | $1.00                       |
| text-embedding-3-large | $0.13          | $6.50                       |
| text-embedding-ada-002 | $0.10          | $5.00                       |

### Ollama (Free, Local)

| Model             | GPU VRAM | Tokens/sec |
| ----------------- | -------- | ---------- |
| nomic-embed-text  | 1.5 GB   | 500        |
| mxbai-embed-large | 2 GB     | 300        |
| all-minilm        | 0.5 GB   | 1000       |

---

## Troubleshooting

### Dimension Mismatch Error

```
Error: Vector dimension 768 does not match index dimension 1536
```

**Cause**: Embedding model changed without rebuilding index.

**Solution**:

```bash
curl -X POST http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID/rebuild-embeddings
```

### Out of Memory (Ollama)

```
Error: CUDA out of memory
```

**Solution**: Use smaller model or reduce batch size:

```bash
ollama pull all-minilm  # Smaller model
```

### Rate Limiting (OpenAI)

```
Error: Rate limit exceeded
```

**Solution**: EdgeQuake automatically retries with backoff. For high throughput:

- Use Tier 2+ OpenAI account
- Or use local Ollama for embedding

---

## Typed ANN registry key (ingest = query)

Ingest and typed ANN **must share the same** `embedding_models(name, dimensions)`
key as the workspace embedder. EdgeQuake never searches another model’s vector
space when the preferred key misses.

| Concern | Source | Role |
| ------- | ------ | ---- |
| Which embedder process loads | `EDGEQUAKE_EMBEDDING_PROVIDER` / provider setup | Runtime client construction — **not** the ANN registry key |
| ANN / typed write+read registry name | Workspace lineage model, else `embedding_model_key_from_env()` (storage SSOT) | `embedding_models.name` + dimensions; empty Compose `EDGEQUAKE_EMBEDDING_MODEL=` falls through to the product default |
| Query filter | `QueryEmbeddings.model` → `MetadataFilter.embedding_model` | Preferred-only via `serving_embedding_model_candidates`; miss → empty ANN (then graph label/seed admit) |

**Rules**

1. Stamp every typed upsert with the active embedder / lineage model name.
2. When `MetadataFilter.embedding_model` (workspace/lineage) is set, typed ANN
   searches **only** that `embedding_models(name, dimensions)` key. A miss is
   empty — never fall through into the process env model’s space.
3. Empty env (`EDGEQUAKE_EMBEDDING_MODEL=` from Compose `:-`) must resolve through
   the SSOT helper — never treat `Ok("")` as a distinct registry name.
4. When rows were written under the wrong name, **rename/backfill** the registry
   and typed tables — see
   [Embedding registry audit & backfill](/docs/operations/embedding-registry-backfill/).

---

## Best Practices

1. **Consistency**: Use same embedding model for entire workspace
2. **Match Dimensions**: Ensure workspace dimension matches model output
3. **Same ANN key**: Ingest and query must share `embedding_models(name, dim)`
4. **Batch When Possible**: Reduce API calls by batching texts
5. **Monitor Costs**: Track embedding token usage in cost dashboard
6. **Consider Local**: Use Ollama for sensitive data or high volume
7. **Test Before Switching**: Compare quality before changing models
8. **Index Optimization**: Tune HNSW parameters for your workload

---

## See Also

- [Vector Search](/docs/deep-dives/vector-storage/) - How similarity search works
- [Configuration Reference](/docs/operations/configuration/) - All embedding settings
- [Performance Tuning](/docs/operations/performance-tuning/) - Optimization guide
- [Embedding registry audit & backfill](/docs/operations/embedding-registry-backfill/) - Ops SQL for model-key mismatches
