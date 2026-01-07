# OODA Loop 7 - Observe

## Quality Improvement Focus

Loops 1-6 focused on:
- Performance (IDF optimization, benchmarks)
- Flexibility (presets, enhanced tokenization)
- API integration (env var config)

Loop 7 focuses on **query-document matching quality**.

## Current Matching Approach

The BM25 reranker uses bag-of-words matching:
- Tokenize query and documents
- Count term frequencies
- Compute BM25 scores with IDF weighting

## Potential Quality Improvements

1. **Phrase matching**: Boost score when query terms appear adjacent
2. **Proximity scoring**: Boost documents where query terms are close together
3. **Query expansion**: Add synonyms or related terms
4. **N-gram matching**: Match 2-grams or 3-grams for better precision
5. **Case-sensitive matching**: Preserve case for acronyms (API, RAG, LLM)

## Current Limitations

Looking at real-world queries in SOTA engine:
- "What are the key entities in the document?"
- "How does X relate to Y?"
- "Summarize the knowledge about Z"

These benefit from:
- Phrase preservation ("key entities" should match together)
- Semantic understanding (handled by embedding layer, not BM25)

## Scope Consideration

BM25 is a reranker - it refines embedding-based results. 
It shouldn't duplicate semantic understanding.

Best fit for BM25 improvement: **Phrase/proximity boosting**
