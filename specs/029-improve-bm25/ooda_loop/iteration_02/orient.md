# OODA Loop 2: Orient

## Technical Options Analysis

### Option A: Replace tokenize() with NFKD-based normalization
**Approach**: Use `unicode_normalization::UnicodeNormalization::nfkd()` to:
1. Decompose characters into base + combining marks
2. Filter out combining marks (diacritics)
3. Result: Universal accent normalization

**Pros**:
- Handles ALL Unicode accents, not just French
- Standards-based (Unicode Technical Report #15)
- No maintenance burden for new languages

**Cons**:
- Slightly more computation per token

### Option B: Add Porter2 Stemming
**Approach**: Use `rust-stemmers::Stemmer` with configurable algorithm

**Pros**:
- Improves recall by 15-30% for morphological variants
- Supports 17+ languages including English, French, German
- Industry-standard Snowball algorithm

**Cons**:
- May reduce precision slightly (overgeneralization)
- Per-token overhead (~50ns per stem)

### Option C: Stop Word Filtering
**Approach**: Filter common high-frequency words

**Pros**:
- Reduces noise in IDF calculations
- Fewer terms = faster scoring

**Cons**:
- Some stop words carry meaning in phrases ("to be or not to be")

## Selected Architecture

Implement a configurable `TokenizerConfig` struct:

```rust
pub struct TokenizerConfig {
    pub enable_stemming: bool,
    pub stemmer_algorithm: Algorithm,  // English, French, etc.
    pub enable_stop_words: bool,
    pub min_token_length: usize,
}
```

**Key Design Decisions**:
1. **Backward Compatibility**: `BM25Reranker::new()` uses `TokenizerConfig::minimal()` (no stemming/stop words)
2. **Opt-in Enhancement**: `BM25Reranker::new_enhanced()` uses full config
3. **Configurable**: `with_tokenizer_config()` builder pattern
4. **Static Fallback**: Keep `tokenize()` as static method for tests
