# OODA Loop 12: Orient - BM25 Reranker Design

## Architecture Decision

### Why BM25 Over MockReranker

```
┌─────────────────────────────────────────────────────────────────────┐
│                    RERANKER COMPARISON                              │
├──────────────────────┬───────────────────┬──────────────────────────┤
│      Feature         │   MockReranker    │      BM25Reranker        │
├──────────────────────┼───────────────────┼──────────────────────────┤
│ IDF Weighting        │        ❌         │          ✅              │
│ Term Frequency       │        ❌         │          ✅              │
│ Length Normalization │        ❌         │          ✅              │
│ Rare Term Boost      │        ❌         │          ✅              │
│ Common Word Penalty  │        ❌         │          ✅              │
│ Requires Training    │        ❌         │          ❌              │
│ External API         │        ❌         │          ❌              │
│ Local / Fast         │        ✅         │          ✅              │
└──────────────────────┴───────────────────┴──────────────────────────┘
```

## BM25 Algorithm Components

### 1. Inverse Document Frequency (IDF)

**Purpose**: Rare terms are more informative than common terms

```
IDF(qi) = ln((N - n(qi) + 0.5) / (n(qi) + 0.5) + 1)

Where:
- N = total documents
- n(qi) = documents containing term qi
```

**Example**:
- "2008" appears in 1/4 docs → IDF ≈ 1.1
- "Peugeot" appears in 4/4 docs → IDF ≈ 0.3

### 2. Term Frequency Saturation

**Purpose**: Diminishing returns for repeated term mentions

```
TF_component = f(qi, D) × (k1 + 1) / (f(qi, D) + k1)

Where k1 = 1.2 (default)
```

**Example**:
- TF=1 → saturation = 1.1/2.2 = 0.55
- TF=5 → saturation = 5.5/6.2 = 0.89
- TF=10 → saturation = 10.1/11.2 = 0.90

### 3. Length Normalization

**Purpose**: Long documents shouldn't dominate short focused ones

```
norm = 1 - b + b × (|D| / avgdl)

Where b = 0.75 (default)
```

**Example** (avgdl = 200):
- 100 word doc: norm = 0.25 + 0.75 × 0.5 = 0.625
- 200 word doc: norm = 0.25 + 0.75 × 1.0 = 1.0
- 400 word doc: norm = 0.25 + 0.75 × 2.0 = 1.75

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                         BM25 RERANKING FLOW                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   Input:                                                             │
│   ┌──────────────────────┐                                          │
│   │ Query: "2008 ENVY"   │                                          │
│   │ Docs: [d1, d2, d3..]│                                          │
│   └──────────┬───────────┘                                          │
│              ▼                                                       │
│   Step 1: Tokenization                                              │
│   ┌──────────────────────┐                                          │
│   │ query_terms: [2008, envy]                                       │
│   │ doc_terms: [[...], [...]]                                       │
│   └──────────┬───────────┘                                          │
│              ▼                                                       │
│   Step 2: Compute IDF (across all docs)                             │
│   ┌──────────────────────┐                                          │
│   │ IDF("2008") = 0.69   │                                          │
│   │ IDF("envy") = 1.39   │                                          │
│   └──────────┬───────────┘                                          │
│              ▼                                                       │
│   Step 3: Compute BM25 Score per Doc                                │
│   ┌──────────────────────┐                                          │
│   │ doc1: 2.31           │                                          │
│   │ doc2: 0.45           │                                          │
│   │ doc3: 1.12           │                                          │
│   └──────────┬───────────┘                                          │
│              ▼                                                       │
│   Output: Ranked docs by BM25 score                                 │
│   ┌──────────────────────┐                                          │
│   │ [doc1, doc3, doc2]   │                                          │
│   └──────────────────────┘                                          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Struct Definition

```rust
pub struct BM25Reranker {
    /// k1: Term frequency saturation parameter
    k1: f64,
    /// b: Length normalization parameter
    b: f64,
    /// Whether to apply Unicode normalization
    normalize_unicode: bool,
}
```

### Key Methods

1. `tokenize(text: &str) -> Vec<String>` - Unicode-aware tokenization
2. `compute_idf(term: &str, docs: &[&str]) -> f64` - IDF calculation
3. `compute_bm25(query: &str, doc: &str, avgdl: f64, idf: &HashMap) -> f64`
4. `rerank(query, docs, top_n) -> Vec<RerankResult>` - Main entry point

## Test Plan

| Test Case | Query | Expected First | Metric |
|-----------|-------|----------------|--------|
| Exact match | "2008 ENVY" | peugeot-2008-envy.md | score > 2.0 |
| Similar numbers | "2008" | 2008 doc (not 208) | 2008 >> 208 |
| French terms | "motorisation" | Multiple docs | recall >= 3 |
| Rare term | "ENVY" | Only 2008-envy | IDF boost |

## Next Step
Implement BM25Reranker in Rust
