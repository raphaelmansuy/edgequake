# OODA Loop 3: Observe

## Current State

After OODA Loop 2, the enhanced tokenizer is available but only used when:
1. Creating `BM25Reranker::new_enhanced()` explicitly
2. Calling `with_tokenizer_config(TokenizerConfig::enhanced())`

### API Layer Usage
The `state.rs` file creates BM25Reranker in three places:
- Line 290: `new_memory()` constructor
- Line 551: `new_postgres()` constructor

Both used `BM25Reranker::new()` which doesn't enable stemming or stop words.

### Problem
Users get the minimal tokenizer by default, missing out on:
- 15-30% recall improvement from stemming
- Better Unicode accent handling
- Stop word noise reduction

## Observations

1. **No configuration path**: API didn't expose a way to enable enhanced tokenization
2. **Duplicated code**: Same BM25Reranker creation logic in 2+ places
3. **No logging**: No visibility into which tokenizer mode is active
