# OODA Loop Iteration 01: Observe

## Date: 2026-01-07

## Current State Analysis

### 1. BM25 Implementation Location

The primary BM25 implementation is located at:
- [edgequake/crates/edgequake-llm/src/reranker.rs](../../../edgequake/crates/edgequake-llm/src/reranker.rs#L658-L815) (1969 lines total)

### 2. BM25 Components Found

```
BM25Reranker     - Core reranker with k1=1.5, b=0.75, delta=0 (standard BM25)
BM25Reranker+    - Extension with delta=1.0 for long document handling  
HybridReranker   - Combines BM25 + vector similarity via RRF
RRFReranker      - Reciprocal Rank Fusion for combining rankings
MockReranker     - Term overlap fallback (TermOverlapReranker alias)
HttpReranker     - External API rerankers (Jina, Cohere, Aliyun)
```

### 3. Test Coverage Baseline

```
Total BM25-related tests: 61
Passing: 61 (100%)
Performance: 1000 docs reranked in <1s
```

### 4. Key Observations

#### Strengths of Current Implementation:
1. **SOTA-compliant**: Uses Robertson et al. formulas with correct IDF
2. **BM25+ extension**: Supports delta parameter for long documents
3. **Configurable parameters**: k1, b, delta all customizable
4. **Unicode support**: French accent normalization included
5. **Well-tested**: 55+ unit tests covering edge cases
6. **Hybrid support**: RRF fusion with vector rankings

#### Potential Improvement Areas:
1. **No tantivy integration**: Custom in-memory BM25, not using tantivy's optimized implementation
2. **Re-tokenization on every query**: No inverted index, O(n*m) complexity
3. **No stemming**: "running" won't match "run"
4. **Limited Unicode normalization**: Only French accents handled
5. **No stop word removal**: Common words counted equally
6. **No query expansion**: No synonyms, no semantic fallback

### 5. Architecture Flow

```
Query → BM25Reranker.rerank()
         ↓
   tokenize(query)     ← Custom tokenizer, lowercase + accent normalize
         ↓
   tokenize(documents) ← Re-tokenizes ALL docs every call (inefficient)
         ↓
   compute_idf(terms)  ← IDF cache per-query (not persisted)
         ↓
   compute_bm25_score(query_terms, doc_terms)
         ↓
   sort by score
         ↓
   truncate to top_n
```

### 6. Where BM25 is Used

```
sota_engine.rs:206   - Optional reranker field
sota_engine.rs:245   - with_reranker() builder
sota_engine.rs:291   - rerank_chunks() method calls reranker
engine.rs:92-98      - enable_rerank / rerank_top_k config
chunk_retrieval.rs   - rerank_chunks_by_similarity (vector-based, not BM25)
```

### 7. Tantivy Assessment

Tantivy is a full-text search engine library with:
- Inverted index for O(log n) term lookup
- BM25 scoring built-in (Bm25Weight struct)
- Optimized SIMD compression
- Tokenizers with stemming support
- Memory-mapped directories for persistence

**Key Question**: Should we integrate tantivy for BM25?

**Analysis**:
- Current BM25 is a **reranker** (post-retrieval)
- Tantivy is designed for **primary retrieval** (pre-retrieval)
- Adding tantivy would add ~1.4MB dependency
- Current implementation is fast enough (<1s for 1000 docs)
- Tantivy would require index management overhead

**Verdict**: Tantivy integration may be overkill for reranking use case. Focus on improving current implementation first.

### 8. Files & Line Counts

| File | Lines | Purpose |
|------|-------|---------|
| reranker.rs | 1969 | BM25, RRF, Hybrid, HTTP rerankers |
| sota_engine.rs | 2004 | Query engine with reranker integration |
| engine.rs | 627 | Query engine config |
| chunk_retrieval.rs | 325 | Vector-based chunk reranking |
| graph.rs | 1784 | PostgreSQL full-text search (ts_vector) |

## Data Gathered

- All 61 BM25-related tests pass
- Current BM25 is correct algorithmically
- Main inefficiency is O(n) re-tokenization per query
- No stemming or stop word support
- Limited Unicode normalization
