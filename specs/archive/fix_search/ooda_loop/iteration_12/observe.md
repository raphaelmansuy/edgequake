# OODA Loop 12: Observe - MockReranker Limitations Analysis

## Mission

Replace MockReranker with a production-grade reranker that provides better precision and recall.

## Current MockReranker Analysis

```rust
// Current implementation: Simple word overlap
let overlap = query_terms.intersection(&doc_terms).count();
let max_terms = query_terms.len().max(1);
let score = overlap as f64 / max_terms as f64;
```

### Problems Identified

1. **No IDF Weighting**: All terms weighted equally

   - "2008" scores same as "the"
   - Common words dominate scoring

2. **No Term Frequency**:

   - A document mentioning "2008" 10 times scores same as one mention
   - Misses document relevance signals

3. **No Length Normalization**:

   - Long documents have unfair advantage in overlap
   - Short, focused documents penalized

4. **Binary Matching Only**:

   - No partial matching (fuzzy, stemming)
   - "motorisation" won't match "motorisations"

5. **Case Sensitivity Issues**:
   - Already lowercase, but no Unicode normalization
   - "Véhicule" vs "vehicule" handled poorly

## Test Case Failure Analysis

### Query: "2008"

**Expected**: Peugeot 2008 document first
**Problem**: "208" has similar embedding distance, MockReranker helps but is primitive

### Query: "motorisation hybride"

**Expected**: Documents about hybrid powertrains
**Problem**: Term overlap treats "motorisation" and "hybride" equally

## Proposed Solutions

### Option A: BM25 Reranker

- Industry standard for text retrieval
- IDF weighting for rare terms
- Term frequency saturation
- Length normalization

### Option B: RRF (Reciprocal Rank Fusion)

- Combines multiple ranking signals
- No tuning required
- Good for hybrid search

### Option C: BM25 + RRF Hybrid

- Best of both worlds
- BM25 for scoring + RRF for combining with vector similarity

## Research Findings

### BM25 Formula

```
score(D, Q) = Σ IDF(qi) × f(qi, D) × (k1 + 1) / (f(qi, D) + k1 × (1 - b + b × |D| / avgdl))

Where:
- f(qi, D) = term frequency in document
- |D| = document length
- avgdl = average document length
- k1 ∈ [1.2, 2.0] = term frequency saturation
- b = 0.75 = length normalization factor
- IDF(qi) = ln((N - n(qi) + 0.5) / (n(qi) + 0.5) + 1)
```

### RRF Formula

```
score = Σ 1 / (k + rank(result(q), d))

Where:
- k = ranking constant (typically 60)
- rank = position in result list (1-indexed)
```

## Decision Point

Proceed with **BM25Reranker** implementation as the primary improvement, then add RRF as a secondary enhancement.

## Next Steps

1. Implement BM25Reranker in `edgequake-llm/src/reranker.rs`
2. Add comprehensive tests
3. Benchmark against MockReranker
4. Integrate into SOTAQueryEngine
