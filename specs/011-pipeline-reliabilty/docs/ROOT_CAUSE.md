# ROOT CAUSE — Precise Code Path Analysis

> **Spec**: 011-pipeline-reliability  
> **Cross-refs**: [WHY.md](WHY.md) · [IMPROVEMENT_PLAN.md](IMPROVEMENT_PLAN.md) · [EDGE_CASES.md](EDGE_CASES.md)

---

## 1. Failure Taxonomy

| #    | Error Message                                          | Mistral Code | First Fixed  | Status  |
| ---- | ------------------------------------------------------ | ------------ | ------------ | ------- |
| FM-1 | "Too many tokens overall, split into more batches."    | 3210         | spec-010     | ✅ Fixed |
| FM-2 | "Too many inputs in request, split into more batches." | 3210         | **spec-011** | 🔴 Open  |
| FM-3 | Entity extraction JSON EOF (truncated output)          | —            | spec-010     | ✅ Fixed |

---

## 2. FM-2 — Code Path Walkthrough

### 2.1 Call chain

```
API handler: POST /ingest
  └─ TaskWorker::process_task()
       └─ TextInsertProcessor::process()
            └─ Pipeline::process_with_resilience()
                 ├─ resilient_extract_parallel()           ← entity extraction (OK)
                 └─ generate_all_embeddings()              ← FAILS HERE
                      └─ embed_with_token_budget(provider, &entity_texts)
                           └─ embed_batched(sub_batch_of_700_items)
                                └─ embed(700_items)         ← 400 from Mistral
```

### 2.2 `generate_all_embeddings` — where the call is made

File: `edgequake/crates/edgequake-pipeline/src/pipeline/helpers.rs`

```rust
// Entity texts for EU AI Act: ~1 000 items, many 15-40 chars
let all_entity_texts: Vec<String> = extractions
    .iter()
    .flat_map(|e| e.entities.iter().map(|en| format!("{}: {}", en.name, en.description)))
    .collect();
// all_entity_texts.len() ≈ 1 000 for EU AI Act

let all_embeddings = embed_with_token_budget(provider, &safe_entity_texts).await?;
//                   ^^^^^^^^^^^^^^^^^^^^^^^^^^ propagates 400 error as PipelineError::EmbeddingError
```

### 2.3 `embed_with_token_budget` — token-only splitting

```rust
let max_tokens = provider.max_tokens();  // = 8192 for mistral-embed
let token_budget = (max_tokens as f64 * EMBED_SAFETY_FACTOR) as usize;  // = 6963

// Loop over texts
for (i, text) in texts.iter().enumerate() {
    let text_tokens = ((text.len() as f64) / EMBED_CHARS_PER_TOKEN).ceil() as usize;
    //                  ~25 chars / 2.5 = 10 tokens for "ARTICLE_5: ..."

    // Flush if token budget exceeded
    if batch_tokens + text_tokens > token_budget && i > batch_start {
        // FLUSH sub-batch → embed_batched(sub_batch)
    }
    batch_tokens += text_tokens;
}
// With 1000 items × 10 tokens = 10 000 tokens total,
// first sub-batch fills at item ~696 (696 × 10 = 6960 ≤ 6963)
// → sends 696 items to embed_batched
```

### 2.4 `embed_batched` — count-only splitting

```rust
let batch_size = self.max_batch_size();  // = 2048 for Mistral (WRONG)
if texts.len() <= batch_size {           // 696 <= 2048 → true
    return self.embed(texts).await;      // sends all 696 items in one HTTP call
}
```

### 2.5 Mistral HTTP layer — rejects

```
POST https://api.mistral.ai/v1/embeddings
Body: { "model": "mistral-embed", "input": [696 strings] }

400 Bad Request
{"object":"error",
 "message":"Too many inputs in request, split into more batches.",
 "code":"3210"}
```

Mistral's actual input count limit: **≤ 512 inputs per request** (undocumented in primary docs but confirmed via error code 3210 with this message).

---

## 3. FM-2 Root Cause Summary

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ROOT CAUSE: Two independent Mistral limits treated as one dimension        │
│                                                                             │
│  KNOWN limits for mistral-embed:                                            │
│    (A) total tokens per request ≤ 8 192     → fixed by token-budget split   │
│    (B) input count per request  ≤ 512       → NOT enforced by our code      │
│                                                                             │
│  For small documents:  few entities → count stays under 512 → no error     │
│  For EU AI Act:        many short entities → count exceeds 512 → 400 error │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Contributing Factors

### 4.1 Mistral `max_batch_size()` not overridden

`MistralProvider` (edgequake-llm 0.6.14) does not override `EmbeddingProvider::max_batch_size()`.  
The trait default reads `EDGEQUAKE_EMBEDDING_BATCH_SIZE` env var or returns 2048.  
Actual Mistral limit is 512 — a 4× discrepancy.

### 4.2 `SafetyLimitedEmbeddingProviderWrapper` delegates blindly

```rust
fn max_batch_size(&self) -> usize {
    self.inner.max_batch_size()   // just proxies → still 2048
}
```

The safety wrapper wraps the LLM provider for timeouts and token capping, but adds no capping on batch count. Adding a configurable cap here would be the right place for a defence-in-depth fix.

### 4.3 No embedding-layer retry

When `embed()` returns `Err(400)`, `embed_with_token_budget` propagates it immediately. There is no retry-with-smaller-batch logic at the embedding layer. The task worker's outer retry (max 3 attempts) retries the entire document — which still sends the same oversized batch on attempt 2, producing the same 400.

```
TaskWorker retry loop:
  Attempt 1 → embed(696 items) → 400 → fail
  Attempt 2 → embed(696 items) → 400 → fail   ← same batch, same error
  Attempt 3 → embed(696 items) → 400 → fail
  → document permanently FAILED
```

---

## 5. Why This Was Not Caught Earlier

| Reason                                       | Detail                                                   |
| -------------------------------------------- | -------------------------------------------------------- |
| Test docs are small                          | Synthetic and test PDFs have < 300 total entities        |
| Legal docs differ structurally               | High entity density per token (many short terms)         |
| Error was masked                             | spec-010 fix for "Too many tokens" worked → tests passed |
| No integration test with realistic large doc | E2E tests use small PDFs                                 |

---

## 6. Cross-references

| Related item                            | File                                          | Impact                  |
| --------------------------------------- | --------------------------------------------- | ----------------------- |
| `embed_with_token_budget`               | `pipeline/helpers.rs`                         | Primary fix target      |
| `SafetyLimitedEmbeddingProviderWrapper` | `api/safety_limits.rs`                        | Secondary fix target    |
| `MistralProvider::max_batch_size`       | edgequake-llm 0.6.14 (external)               | Source of wrong default |
| FM-1 token splitting                    | `pipeline/helpers.rs:embed_with_token_budget` | Prior art for the fix   |
