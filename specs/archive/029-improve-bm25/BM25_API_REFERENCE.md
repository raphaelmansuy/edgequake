# BM25 Reranker - API Reference

## Overview

The BM25Reranker implements the Okapi BM25 ranking algorithm with EdgeQuake enhancements:

- **BM25+**: Extended scoring to prevent long document penalization
- **Enhanced tokenization**: Porter2 stemming, Unicode normalization, stop word filtering
- **Phrase boosting**: Bonus for adjacent query term matches
- **Domain presets**: Pre-configured parameters for different use cases

## Quick Start

```rust
use edgequake_llm::BM25Reranker;

// Standard BM25 (backward compatible)
let reranker = BM25Reranker::new();

// Enhanced BM25 with stemming
let reranker = BM25Reranker::new_enhanced();

// Domain-specific presets
let reranker = BM25Reranker::for_rag();        // RAG queries
let reranker = BM25Reranker::for_semantic();   // Phrase-sensitive
let reranker = BM25Reranker::for_technical();  // Code/API docs
let reranker = BM25Reranker::for_short_docs(); // Tweets/titles
let reranker = BM25Reranker::for_long_docs();  // Papers/articles

// Rerank documents
let results = reranker.rerank("query text", &documents, Some(10)).await?;
```

## Constructors

### `new()`

Standard BM25 reranker with minimal tokenization (no stemming, no stop words).

- **k1**: 1.5
- **b**: 0.75
- **delta**: 0.0
- **phrase_boost**: 0.0

### `new_enhanced()`

Enhanced BM25 with stemming and stop word filtering.

- Same parameters as `new()` but with enhanced tokenization

### `bm25_plus()`

BM25+ variant with delta=1.0 for better long document handling.

### `for_rag()`

Optimized for RAG/knowledge graph queries:

- **delta**: 0.5 (mild BM25+)
- **phrase_boost**: 0.3 (moderate)
- Enhanced tokenization

### `for_semantic()`

Optimized for phrase-sensitive queries:

- **phrase_boost**: 0.5 (stronger)
- Enhanced tokenization

### `for_technical()`

Optimized for code and API documentation:

- **k1**: 2.0 (higher term frequency saturation)
- **b**: 0.5 (moderate length normalization)
- Minimal tokenization (no stemming for exact matches)

### `for_short_docs()`

Optimized for short content (tweets, titles):

- **k1**: 1.2 (lower saturation)
- **b**: 0.3 (reduced length penalty)

### `for_long_docs()`

Optimized for long documents (papers, articles):

- **delta**: 1.0 (full BM25+)
- Standard parameters

## Builder Methods

### `with_params(k1, b)`

Custom k1 and b parameters.

### `with_full_params(k1, b, delta)`

Custom parameters including BM25+ delta.

### `with_phrase_boost(boost)`

Add phrase boosting (0.0-2.0).

### `with_tokenizer_config(config)`

Custom tokenizer configuration.

## Environment Variables

- `BM25_ENHANCED=true|false`: Enable/disable enhanced tokenization (default: true)

## Algorithm Details

### BM25 Scoring Formula

```
score(D,Q) = Σ IDF(qᵢ) × (f(qᵢ,D) × (k1 + 1)) / (f(qᵢ,D) + k1 × (1 - b + b × |D|/avgdl)) + δ
```

Where:

- f(qᵢ,D): Term frequency of query term in document
- |D|: Document length
- avgdl: Average document length
- k1: Term frequency saturation (default 1.5)
- b: Length normalization (default 0.75)
- δ: BM25+ extension (default 0 for standard, 1 for BM25+)

### Phrase Boosting

When phrase_boost > 0:

```
final_score = bm25_score + (phrase_boost × phrase_bonus)
```

Where phrase_bonus is the proportion of consecutive query term pairs
found adjacent in the document.

### Enhanced Tokenization

1. **Unicode normalization** (NFKD): Decomposes accents (café → cafe)
2. **Porter2 stemming**: Reduces words to stems (running → run)
3. **Stop word filtering**: Removes high-frequency words (the, and, is)
4. **Minimum length**: Filters tokens < 2 characters
