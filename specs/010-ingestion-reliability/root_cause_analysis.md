# Ingestion Reliability — Root Cause Analysis

> **Status**: Fixed in v0.11.2  
> **Branch**: `fix/010-ingestion-reliability`  
> **Affected version**: ≤ v0.11.1

---

## Executive Summary

Two independent production failure modes were observed during document ingestion with the
Mistral provider on dense technical PDFs (seismology / scientific literature).  Both failures
share the same root cause pattern: **a hard-coded limit applied to an operation whose actual
output size scales with document complexity**.

---

## Failure Mode 1 — Entity Extraction JSON EOF

### Symptoms

```
WARN  edgequake_pipeline::pipeline::extraction: Chunk extraction failed, will retry
        chunk_index=120 attempt=2 max_retries=3
        error=Entity extraction error: Invalid JSON: EOF while parsing a list at line 179 column 215

WARN  edgequake_api::safety_limits: Safety limit: max_tokens clamped to configured limit
        requested_tokens=16384 enforced_tokens=8192
```

The error always appears on the **last retry attempt** (attempt 3 of 3), never on attempt 1.
This is a strong signal that the failure is **deterministic** — retrying the same call with the
same budget produces the same truncation.

### Call Chain

```
LLMExtractor::extract(chunk)
  └─ complete_with_options(prompt, { max_tokens: 16384 })
       └─ SafetyLimitedProviderWrapper::complete_with_options(...)
            └─ apply_token_limit()          ← CLAMPS 16384 → 8192
                 └─ inner.complete_with_options(prompt, { max_tokens: 8192 })
                      └─ Mistral API → truncated JSON response
```

### Root Cause

`SafetyLimitedProviderWrapper` enforced `DEFAULT_MAX_TOKENS = 8192`.  The LLM extractor
requested `max_tokens = 16384`, which was silently clamped.  When a dense chunk contains
100+ entities, the full JSON response requires more than 8 192 output tokens.  The model
is cut off mid-array, producing invalid JSON that cannot be parsed.

All three retry attempts hit the same wall because the token budget never escalates — the
retry loop reuses the same `LLMExtractor::extract` path unchanged.

### Why This Was Not Caught Earlier

- Small/medium documents generate < 8 192 output tokens per chunk
- The safety limit warning was logged at `WARN` but not surfaced as an extraction failure
- Integration tests used short synthetic texts that never exceeded the budget

---

## Failure Mode 2 — Embedding "Too Many Tokens"

### Symptoms

```
ERROR edgequake_api::processor::text_insert: CRITICAL: Pipeline processing failed
  error=Embedding error: API error: Mistral embeddings API error (400 Bad Request):
  {"object":"error","message":"Too many tokens overall, split into more batches.",
   "type":"invalid_request_prompt","param":null,"code":"3210","raw_status_code":400}

DEBUG edgequake_pipeline::pipeline::helpers: Linking 135 entities and 31 relationships to chunk
DEBUG edgequake_llm::providers::mistral: Mistral embed request model="mistral-embed" count=142
```

The pipeline crashes fatally — the task is permanently marked as failed.  No partial results
are stored.

### Call Chain

```
Pipeline::generate_all_embeddings(...)
  └─ provider.embed_batched(&all_entity_texts)   ← 142 texts in ONE call
       └─ EmbeddingProvider::embed_batched()
            └─ max_batch_size() → 2048           ← count-only, 142 < 2048, no split
                 └─ self.embed(all_142_texts)
                      └─ Mistral POST /v1/embeddings   ← total tokens > 8192 → 400
```

### Root Cause

`EmbeddingProvider::embed_batched` (in `edgequake-llm` traits) splits inputs by **count**
only (`max_batch_size`, default 2 048).  `MistralProvider` does not override
`max_batch_size`, so 142 entity strings are sent in a single HTTP request.

Mistral's `mistral-embed` model enforces an **8 192-token TOTAL budget per request**
(not per string).  For dense seismology text, entity descriptions can average 80–200 chars
each.  At 1.5–2.5 chars/token (dense abbreviations, gene IDs, coordinates):

```
142 texts × 100 chars avg / 1.8 chars/token ≈ 7 889 tokens  →  borderline
142 texts × 120 chars avg / 1.5 chars/token ≈ 11 360 tokens →  400 error
```

### Why The 400 Is Fatal

Unlike extraction (which is retried), the embedding error is wrapped in
`PipelineError::EmbeddingError` and propagated as a hard failure because:
1. No retry logic exists in `generate_all_embeddings`
2. The error originates in the HTTP layer, not the extraction layer
3. No partial success handling exists for embedding batches

---

## Relationship Between The Two Failures

Both failures stem from the same root pattern:

```
╔════════════════════════════════════════════════════════════════════╗
║  A hard-coded limit (token cap, batch count) was set for          ║
║  "average" documents. Dense technical PDFs push outputs beyond    ║
║  that limit on every attempt. No adaptive mitigation existed.     ║
╚════════════════════════════════════════════════════════════════════╝
```

| Property           | Failure 1 (Extraction)          | Failure 2 (Embedding)            |
| ------------------ | ------------------------------- | -------------------------------- |
| Trigger            | Chunk with > 100 entities       | Batch of > ~80 entity texts      |
| Limit violated     | Output token cap (8 192)        | Total batch token budget (8 192) |
| Where limit lives  | `safety_limits.rs` (API crate)  | Mistral API (external)           |
| Failure mode       | JSON truncation → parse error   | HTTP 400 → hard crash            |
| Retry behavior     | 3 retries, all fail identically | No retry (hard fail)             |
| Documents affected | Dense PDFs, scientific papers   | Same (many entities)             |
