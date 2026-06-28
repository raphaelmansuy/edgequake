# IMPROVEMENT PLAN — Bulletproof Ingestion Pipeline

> **Spec**: 011-pipeline-reliability  
> **Cross-refs**: [WHY.md](WHY.md) · [ROOT_CAUSE.md](ROOT_CAUSE.md) · [EDGE_CASES.md](EDGE_CASES.md) · [TEST_RESULTS.md](TEST_RESULTS.md)

---

## 1. Objectives

Make the ingestion pipeline succeed reliably for:
- Documents up to 500 000 characters
- Dense legal/regulatory texts (EU AI Act, GDPR, ISO standards)
- Dense scientific papers (many entity names, dense relationships)
- Any Mistral-hosted embedding provider

---

## 2. Fix Architecture

```
  Current (broken for EU AI Act)          Fixed (dual-dimension guard)
  ─────────────────────────────           ────────────────────────────

  embed_with_token_budget                  embed_with_token_budget
    │                                        │
    ├─ split by TOKEN BUDGET only            ├─ split by TOKEN BUDGET
    │   (6963 tokens) ✓                      │   AND
    │                                        ├─ split by INPUT COUNT
    └─ embed_batched(N items)                │   (provider.max_batch_size() → 512)
         │                                   │
         └─ max_batch_size() = 2048          └─ embed_batched(≤ 512 items)
              → sends N > 512 items               → embed(≤ 512 items)  ✓
              → Mistral 400 ✗
```

---

## 3. Changes Required

### 3.1 Fix A — `embed_with_token_budget` (PRIMARY)

**File**: `edgequake/crates/edgequake-pipeline/src/pipeline/helpers.rs`

**Change**: Add a second flush condition: when the current sub-batch count would
exceed `provider.max_batch_size()`.

```rust
// BEFORE (token-only splitting):
if batch_tokens + text_tokens > token_budget && i > batch_start {
    flush_batch()
}

// AFTER (dual token + count splitting):
let max_count = provider.max_batch_size();
let count_would_exceed = (i - batch_start) >= max_count;
if (batch_tokens + text_tokens > token_budget || count_would_exceed) && i > batch_start {
    flush_batch()
}
```

**Why this fixes EC-001**: Sub-batches are now bounded by BOTH token budget and input count.  
With `max_batch_size()` returning 512 (after Fix B), no sub-batch exceeds 512 items.

### 3.2 Fix B — `SafetyLimitsConfig` embedding batch cap (SECONDARY)

**File**: `edgequake/crates/edgequake-api/src/safety_limits.rs`

**Change**: Add `max_embed_batch_size` field to `SafetyLimitsConfig` (default: 512).  
Override `max_batch_size()` in `SafetyLimitedEmbeddingProviderWrapper` to enforce this cap.

```rust
// SafetyLimitsConfig addition:
pub max_embed_batch_size: usize,  // default: 512

// SafetyLimitedEmbeddingProviderWrapper:
fn max_batch_size(&self) -> usize {
    let inner = self.inner.max_batch_size();
    inner.min(self.config.max_embed_batch_size)
}
```

**Why 512**: Mistral's actual input count limit per request.  
**Why configurable**: Other providers (OpenAI, Ollama TEI) may support larger batches.  
**Env var**: `EDGEQUAKE_EMBEDDING_BATCH_SIZE` (already supported by trait default, but now also enforced at wrapper level).

### 3.3 Fix C — Embedding error classification (TERTIARY)

**File**: `edgequake/crates/edgequake-api/src/processor/text_insert.rs` (or equivalent)

**Change**: Classify embedding errors before propagating:
- 400 Bad Request → retriable with smaller batch (already fixed by A+B)  
- 429 Too Many Requests → retry with exponential backoff  
- 5xx → retry (may recover)  
- auth errors → permanent failure

**Scope**: Fix A+B prevent the 400 from occurring at all. Fix C is a defence-in-depth fallback.

---

## 4. Implementation Steps

```
- [x] Step 1: Write spec documentation (WHY, ROOT_CAUSE, EDGE_CASES, IMPROVEMENT_PLAN)
- [x] Step 2: Add max_embed_batch_size to SafetyLimitsConfig (Fix B)
- [x] Step 3: Override max_batch_size() in SafetyLimitedEmbeddingProviderWrapper (Fix B)
- [x] Step 4: Add count-based splitting to embed_with_token_budget (Fix A)
- [x] Step 5: Update existing tests (CountingEmbedProvider + new_with_batch constructor)
- [x] Step 6: Add new tests: count-limit splitting, dual-limit, boundary conditions
- [x] Step 7: Run full test suite (203 tests pass, 0 fail)
- [x] Step 8: Update TEST_RESULTS.md with results
```

---

## 5. Test Scenarios

### 5.1 Unit tests (in helpers.rs)

| Test                       | Input                               | Expected                      |
| -------------------------- | ----------------------------------- | ----------------------------- |
| count_limit_splits_batches | 600 texts, max_batch_size=512       | ≥ 2 embed() calls, each ≤ 512 |
| dual_limit_token_wins      | 100 long texts, tight token budget  | splits at token budget        |
| dual_limit_count_wins      | 600 short texts, large token budget | splits at count limit         |
| boundary_exactly_512       | 512 texts                           | exactly 1 embed() call        |
| boundary_513               | 513 texts                           | exactly 2 embed() calls       |

### 5.2 Integration test

| Test                   | Input                                       | Expected                          |
| ---------------------- | ------------------------------------------- | --------------------------------- |
| large_legal_doc_ingest | EU AI Act (231 764 chars) via mock provider | Completes without embedding error |

### 5.3 `SafetyLimitedEmbeddingProviderWrapper` tests

| Test                    | Config                    | Input                          | Expected    |
| ----------------------- | ------------------------- | ------------------------------ | ----------- |
| max_batch_size_clamped  | max_embed_batch_size=16   | provider.max_batch_size()=2048 | returns 16  |
| max_batch_size_uses_min | max_embed_batch_size=1024 | provider.max_batch_size()=512  | returns 512 |

---

## 6. Acceptance Criteria

- [x] `embed_with_token_budget` splits when EITHER token budget OR count limit is exceeded
- [x] `SafetyLimitedEmbeddingProviderWrapper::max_batch_size()` enforces a configurable cap
- [x] Default cap is 512 (matching Mistral's undocumented limit)
- [x] All existing `embed_with_token_budget` unit tests pass
- [x] New count-limit tests pass
- [x] `cargo test --workspace --lib` passes
- [x] `cargo clippy` produces no new warnings

---

## 7. Risk Assessment

| Risk                                       | Likelihood | Impact | Mitigation                                                   |
| ------------------------------------------ | ---------- | ------ | ------------------------------------------------------------ |
| Cap of 512 is wrong for some providers     | Low        | Medium | `EDGEQUAKE_EMBEDDING_BATCH_SIZE` env var override            |
| More sub-batches → more API calls → slower | Medium     | Low    | Batches are still 512 items; cost increase < 2%              |
| New split creates out-of-order embeddings  | Low        | High   | Tests verify ordered result assembly                         |
| Cap too conservative for OpenAI            | Low        | Low    | OpenAI default 2048 > 512; wrapper uses min() so 512 applied |

---

## 8. See Also

- [TEST_RESULTS.md](TEST_RESULTS.md) — Actual test outcomes and honest assessment
- Spec 010: `specs/010-ingestion-reliability/` — Prior ingestion fix (FM-1: token count)
- `edgequake/crates/edgequake-pipeline/src/pipeline/helpers.rs` — Primary fix file
- `edgequake/crates/edgequake-api/src/safety_limits.rs` — Secondary fix file
