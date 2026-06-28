# EDGE CASES — Complete Failure Mode Inventory

> **Spec**: 011-pipeline-reliability  
> **Cross-refs**: [WHY.md](WHY.md) · [ROOT_CAUSE.md](ROOT_CAUSE.md) · [IMPROVEMENT_PLAN.md](IMPROVEMENT_PLAN.md)

---

## 1. Edge Case Matrix

```
  Pipeline stage × Document type × Provider limit
  ┌──────────────────┬───────────┬────────────────┬─────────────────────┬──────────┐
  │ Stage            │ Doc type  │ Limit crossed  │ Symptom             │ Status   │
  ├──────────────────┼───────────┼────────────────┼─────────────────────┼──────────┤
  │ Extraction       │ Dense PDF │ Output tokens  │ JSON EOF (truncated)│ ✅ Fixed │
  │ Embedding        │ Dense PDF │ Total tokens   │ 400 "too many tokens│ ✅ Fixed │
  │ Embedding        │ Legal PDF │ Input count    │ 400 "too many inputs│ 🔴 OPEN  │
  │ Embedding        │ Any       │ Rate limit 429 │ Permanent failure   │ 🔴 OPEN  │
  │ Extraction       │ Any       │ Network timeout│ Partial results     │ ✅ OK    │
  │ Chunking         │ Table-heavy│ Large single  │ Oversized chunks    │ 🟡 Warn  │
  │                  │           │ chunk          │                     │          │
  │ Task queue       │ Bulk ingest│ DB connection │ Queue stall         │ 🟡 Warn  │
  └──────────────────┴───────────┴────────────────┴─────────────────────┴──────────┘
```

---

## 2. Detailed Edge Case Analysis

### EC-001 — Embedding: "Too many inputs" (PRIMARY — spec 011)

**Trigger**: Document with high entity count composed of short texts  
**Example**: EU AI Act (1 000+ short legal terms)  
**Condition**: `count(embedding_inputs_per_sub_batch) > 512`  
**Root cause**: `embed_with_token_budget` only splits by token budget, not input count  
**Impact**: Document permanently FAILED, all extraction work discarded  
**Fix**: Add count-based splitting alongside token budget in `embed_with_token_budget`

### EC-002 — Embedding: Rate limit (429)

**Trigger**: Many concurrent document ingestions or high-volume batch  
**Condition**: Mistral rate limit exceeded  
**Current behavior**: 400/429 error propagates immediately as hard failure  
**Fix needed**: Exponential backoff + jitter in embedding retry path  
**Note**: Different from extraction retry — embedding has no retry layer at all

### EC-003 — Extraction: All chunks fail

**Trigger**: Complete provider outage during extraction  
**Current behavior**: `resilient_extract_parallel` collects failures → empty extractions → continues to embedding  
**Embedding step**: empty texts → `embed_with_token_budget([]) → Ok([])` (safe)  
**Result**: Document stored with 0 entities/relationships — silent partial success  
**Fix needed**: Log a WARNING and mark document as `Processed_With_Warnings` status

### EC-004 — Very large single chunk (tables, definitions)

**Trigger**: A single chunk exceeds the extraction context window  
**Example**: A 20-page appendix that is not split because it has no paragraph breaks  
**Current behavior**: LLM extractor truncates input at context window, extracts subset  
**Impact**: Loss of entities in large table blocks  
**Fix needed**: Chunker should hard-split at context limit even within paragraphs

### EC-005 — Unicode in entity names

**Trigger**: Entity names contain non-ASCII characters (CJK, Arabic, legal symbols §, ©)  
**Current behavior**: `truncate_at_char_boundary` handles UTF-8 correctly  
**Status**: ✅ Handled in `guard_for_embedding`

### EC-006 — Empty document after PDF extraction

**Trigger**: PDF is image-only (no OCR), or protected, or corrupt  
**Current behavior**: Pipeline processes 0 chunks → 0 entities → completes with empty result  
**Impact**: Silent success, no error surfaced  
**Fix needed**: Fail-fast with explicit error when content_length < MIN_CONTENT_THRESHOLD

### EC-007 — DB connection exhaustion during bulk insert

**Trigger**: Large document with many entities causes many concurrent DB writes  
**Current behavior**: SQLx connection pool blocks; may timeout  
**Status**: 🟡 Monitor via `DATABASE_URL` pool size; not actively causing failures

### EC-008 — Embedding provider dimension mismatch

**Trigger**: Provider changed (e.g., switched from `text-embedding-3-small` to `mistral-embed` after data already exists)  
**Condition**: Stored vectors have dim 1536, new embeddings have dim 1024  
**Impact**: Vector similarity queries return wrong results (no error)  
**Fix needed**: Dimension validation at startup against stored schema

### EC-009 — Task retry amplification

**Trigger**: A permanent error (e.g., "Too many inputs" 400) causes 3 retries  
**Current behavior**: Each retry re-sends the same request → same error → 3× API calls  
**Fix needed**: Distinguish retriable (429, 5xx, network) from permanent (400) errors; skip retries for 400

---

## 3. Priority Matrix

```
                High impact      Low impact
               ┌────────────────┬───────────────┐
  High prob     │ EC-001 ← FIX   │ EC-003 LOG    │
                │ EC-002 ← FIX   │ EC-009 ← FIX  │
               ├────────────────┼───────────────┤
  Low prob      │ EC-006 WARN    │ EC-004 PLAN   │
                │ EC-008 DETECT  │ EC-007 MONITOR│
               └────────────────┴───────────────┘
```

---

## 4. Spec-011 Scope

This spec addresses **EC-001** (primary), **EC-002** (secondary), and **EC-009** (side fix).

All other edge cases are documented here for awareness but are deferred to future iterations.
