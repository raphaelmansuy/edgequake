# OODA Loop 2: Observe

## Current State Analysis

### Tokenizer Implementation
- Static `tokenize()` method used hardcoded French accent mappings
- No stemming support (morphological variants not matched)
- No stop word filtering (high-frequency noise terms included)
- Unicode normalization was incomplete (only French accents)

### Code Location
- `edgequake/crates/edgequake-llm/src/reranker.rs` lines 880-895
- BM25Reranker struct lines 661-690

### Test Baseline
- 35 BM25 tests passing
- All 168 edgequake-llm tests passing
- All 223 edgequake-query tests passing

## Observations

1. **Unicode Normalization Gap**: Hardcoded French mappings (`é → e`) don't cover:
   - German umlauts (ö, ü, ä)
   - Nordic characters (å, ø, æ)
   - Cyrillic transliterations
   - East Asian compatibility characters

2. **Stemming Gap**: No morphological matching:
   - "running" won't match "run" or "runner"
   - "studies" won't match "study" or "studying"
   - Reduces recall significantly for English queries

3. **Stop Word Impact**: Words like "the", "a", "is" dilute IDF scores

4. **Dependencies Available**:
   - `rust-stemmers` v1.2: Porter2 Snowball stemmer (17+ languages)
   - `unicode-normalization` v0.1: Full NFKD decomposition
