# WHY — First Principles Analysis of Pipeline Failures

> **Spec**: 011-pipeline-reliability  
> **Document**: Root cause by first principles  
> **Status**: Active  
> **Cross-refs**: [ROOT_CAUSE.md](ROOT_CAUSE.md) · [IMPROVEMENT_PLAN.md](IMPROVEMENT_PLAN.md) · [EDGE_CASES.md](EDGE_CASES.md)

---

## 1. The Failure Event

**Document**: `European Union Artificial Intelligence Act_Guide_202504 (6).pdf`  
**Size**: 231 764 chars (dense legal text, many defined terms)  
**Provider**: Mistral (`mistral-embed` + `mistral-small` or similar)  
**Error**:

```
CRITICAL: Pipeline processing failed
error = Embedding error: API error: Mistral embeddings API error (400 Bad Request):
  {"object":"error",
   "message":"Too many inputs in request, split into more batches.",
   "type":"invalid_request_prompt",
   "code":"3210",
   "raw_status_code":400}
```

The task was retried once (retry_count=1) before being permanently marked failed.

---

## 2. First Principles Decomposition

### 2.1 What is the pipeline trying to do?

```
PDF / Text Input
      │
      ▼
  ┌─────────────┐
  │   CHUNKER   │  Split document into overlapping windows
  └──────┬──────┘
         │  N chunks (e.g. 200 for 231 764 chars)
         ▼
  ┌─────────────┐
  │  EXTRACTOR  │  LLM → extract entities + relationships per chunk
  └──────┬──────┘
         │  M total entities (e.g. 1000+ for EU AI Act)
         ▼
  ┌──────────────┐
  │  EMBEDDER    │  Embed every chunk text + entity text + relation text
  └──────┬───────┘
         │  Float vectors stored in vector DB
         ▼
  ┌──────────────┐
  │  GRAPH STORE │  Upsert entities, relations, chunks
  └──────────────┘
```

### 2.2 Why does embedding fail?

The embedder calls a cloud API (`mistral-embed`). Cloud APIs enforce rate and size limits.  
Mistral's `mistral-embed` enforces **two independent limits per HTTP request**:

| Limit              | Description                        | Hard value   |
| ------------------ | ---------------------------------- | ------------ |
| `max_tokens_total` | Sum of tokens across all inputs    | 8 192 tokens |
| `max_inputs_count` | Number of individual input strings | 512 inputs   |

The existing code (after spec 010 fix) splits only by token count. It does **not** split by input count. For a dense legal document with many short entity names, the token-safe sub-batch may still contain 700+ inputs — exceeding the 512-input limit.

### 2.3 Why is this hard to see without a large document?

| Property                   | Small document | EU AI Act                 |
| -------------------------- | -------------- | ------------------------- |
| Chars                      | < 50 000       | 231 764                   |
| Chunks                     | < 50           | ~193                      |
| Total entities             | < 200          | 1 000+                    |
| Entity avg len             | 80-150 chars   | 15-40 chars (legal terms) |
| Token budget per sub-batch | 6 963 tokens   | 6 963 tokens              |
| Entities per sub-batch     | ≈ 87           | ≈ 700+                    |
| Exceeds 512 limit?         | No             | **Yes**                   |

Legal documents generate many **short** entities (article references, defined terms, org names). The token budget permits hundreds of them per sub-batch, but the input count limit does not.

### 2.4 Why was the spec 010 fix insufficient?

Spec 010 fixed `"Too many tokens overall"` (token-count limit).  
Spec 011 reveals `"Too many inputs in request"` (input-count limit).  
Same error code (3210), different enforcement dimension. The two limits are orthogonal.

```
              Mistral mistral-embed enforcement matrix
              ┌─────────────────────────────────────────┐
              │                 Input count              │
              │        ≤ 512           > 512            │
              ├─────────────┬──────────────────────────-┤
Token count   │ ≤ 8192  ✓ OK   │    ✗ code 3210 (inputs) │
              ├─────────────┼──────────────────────────-┤
              │ > 8192  ✗ code 3210 (tokens)  ✗ both   │
              └─────────────┴──────────────────────────-┘
```

The spec 010 fix placed us in the bottom-left cell. The EU AI Act moved us to the top-right cell.

---

## 3. Why the Architecture Is Vulnerable

### 3.1 Missing Provider Contract

`EmbeddingProvider::max_batch_size()` was intended to express input-count limits. The Mistral provider **does not override** this method, so it returns the default of 2 048. The actual Mistral limit is 512.

This is a provider contract violation: the trait method promises to return "the maximum number of texts per embedding API request" but Mistral's implementation silently returns a wrong value.

### 3.2 Single-dimension splitting in `embed_with_token_budget`

`embed_with_token_budget` splits only along the **token** axis. It then delegates to `embed_batched()` which splits along the **count** axis using `max_batch_size()`. Because `max_batch_size()` returns 2 048, no count-splitting happens.

Two separate guards that each look correct in isolation produce incorrect composed behavior.

```
embed_with_token_budget(texts)
  │
  ├─ Split by token budget → sub-batches respecting 8 192 tokens ✓
  │
  └─ embed_batched(sub_batch)
       │
       └─ if len(sub_batch) <= max_batch_size (2048) → embed(sub_batch) ← sends 700 items!
```

### 3.3 Permanent hard failure on embedding error

`generate_all_embeddings` has no retry loop. When the 400 comes back it propagates immediately as `PipelineError::EmbeddingError` and the document is permanently marked FAILED. No partial results are saved; all extraction work is discarded.

### 3.4 No adaptive response to API error messages

The code does not inspect the error body to distinguish:
- "Too many tokens overall" → reduce batch tokens
- "Too many inputs in request" → reduce batch count  
- "Rate limit exceeded" (429) → backoff and retry

All are treated identically as fatal errors.

---

## 4. First Principles Laws Violated

| Law                                                        | Violation                                                                              |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| **Postel's Law** (be conservative in what you send)        | Sends maximum allowed batch, not minimum safe                                          |
| **Fail-fast on permanent errors, retry on transient ones** | All errors are treated as fatal (no retry)                                             |
| **Single Responsibility**                                  | `embed_with_token_budget` splits tokens but silently relies on caller to handle counts |
| **Make contracts explicit**                                | `max_batch_size()` returns wrong value for Mistral with no override                    |
| **Defence in depth**                                       | Single guard (token count) instead of dual guard (tokens + count)                      |

---

## 5. The Correct Mental Model

An embedding request to Mistral is valid if and only if:

```
total_tokens(inputs) ≤ 8 192
    AND
count(inputs) ≤ 512
```

Both conditions must hold simultaneously.  
The fix must enforce **both** in the splitting logic.

See [ROOT_CAUSE.md](ROOT_CAUSE.md) for the precise code paths.  
See [IMPROVEMENT_PLAN.md](IMPROVEMENT_PLAN.md) for the implementation.
