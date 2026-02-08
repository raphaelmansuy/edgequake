# OODA Iteration 09 - Observe

## Date: 2026-02-08

## Mission Re-Read Confirmation

✅ Re-read mission file `specs/001-reliable-ingestion-mission.md`

Key objectives from mission:

1. Ensure document upload, embedding, and KG building pipeline is functional
2. Use gpt-5-nano as default OpenAI model
3. Battle test with audit documents using OpenAI
4. Test delete document, parallel ingestion, both providers work

## Observations

### 1. System State

**Backend Status:**

- Running and healthy on localhost:8080
- Storage mode: PostgreSQL
- LLM Provider: Ollama (despite OPENAI_API_KEY being set)
- BM25 reranker enabled

**Database Contents:**

- 6 PDF documents uploaded (all status: "completed")
- 0 entities extracted
- 0 chunks created
- 0 relationships created

**Critical Finding:** PDFs are extracted but the ingestion pipeline is NOT completing!

### 2. Root Cause Analysis

**Error from tasks table:**

```
Processing error: Pipeline processing failed: Embedding error:
API error: Ollama API error (400 Bad Request):
{"error":"the input length exceeds the context length"}
```

**Analysis:**

- `nomic-embed-text` has context_length of **2048 tokens**
- Default chunk_size is **1200 tokens**
- But the embedding is likely being called with full documents or batched content
- Ollama embedding model is rejecting inputs that exceed its context window

### 3. Provider Configuration Issue

**Current defaults (hardcoded in `edgequake-core/src/types/multitenancy.rs`):**

```rust
pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";
```

**Problem:** Even though OPENAI_API_KEY is set, the backend defaults to Ollama.
The Makefile says "Using OpenAI provider" but the backend actually uses Ollama.

### 4. Available Models

OpenAI models available and enabled:

- `gpt-5-nano` (recommended, cost-effective)
- `gpt-4.1`, `gpt-4.1-mini`, `gpt-4.1-nano`
- `text-embedding-3-small` (1536 dims)
- `text-embedding-3-large` (3072 dims)

Ollama embedding models:

- `nomic-embed-text` (768 dims, **2048 context**)
- `embeddinggemma` (768 dims, 2048 context)
- `mxbai-embed-large` (1024 dims, **512 context**)

### 5. Audit Findings Status (from Feb 8 audit)

**Tier 1 (Config) - ✅ COMPLETE:**

- max_entities: 60 ✅
- max_chunks: 20 ✅
- max_context_tokens: 30000 ✅

**Tier 2 (Code) - ⚠️ PARTIALLY COMPLETE:**

- BM25Reranker wired up ✅
- VECTOR chunk selection NOT used in query flow ❌
- Round-robin merge exists but needs verification

### 6. Success Criteria Assessment

| Criteria                           | Status | Notes                       |
| ---------------------------------- | ------ | --------------------------- |
| Document upload via UI works       | ⚠️     | PDF extracts but no KG      |
| Document processing completes      | ❌     | Entities fail to extract    |
| Knowledge graph is populated       | ❌     | 0 entities, 0 relationships |
| gpt-5-nano is default OpenAI model | ⚠️     | In config but not used      |
| Ingestion works with Ollama        | ❌     | Context length error        |
| Ingestion works with OpenAI        | ?      | Not tested yet              |
| Delete document works              | ?      | Not tested yet              |
| Parallel ingestion works           | ?      | Not tested yet              |

## Key Files to Investigate

1. `edgequake-core/src/types/multitenancy.rs` - Hardcoded defaults
2. `edgequake-pipeline/src/pipeline.rs` - Ingestion pipeline
3. `edgequake-llm/src/providers/ollama.rs` - Ollama embedding implementation
4. `edgequake-storage/src/adapters/postgres/vector.rs` - Vector storage

## Conclusion

The ingestion pipeline is broken because Ollama's embedding model has a smaller context window than expected. The pipeline is sending content that exceeds 2048 tokens, causing failures.

Two possible fixes:

1. **Quick fix:** Use OpenAI embeddings (8191 token context)
2. **Proper fix:** Truncate/chunk content before embedding with Ollama
