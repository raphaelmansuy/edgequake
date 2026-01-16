# Task Log: 2026-01-16-16-30-OODA-230-chunk-retrieval-fix

## Actions

- Investigated user report of "0 Sources" in Hybrid mode queries
- Traced query flow through handlers/query.rs → sota_engine.rs
- Identified root cause: missing source_chunk_ids logic in `_with_vector_storage` methods
- Fixed `query_local_with_vector_storage` to collect chunk IDs from entities/relationships
- Fixed `query_global_with_vector_storage` with same pattern
- Created e2e_chunk_retrieval.rs with 3 tests
- Verified all 397 API tests pass
- Committed fix as 6eeecb4

## Decisions

- Used "copy logic" approach over "extract shared method" for lower risk
- Added WHY-OODA230 comments for future maintainers
- Did not fix Naive mode (different design intent: direct semantic search)

## Next Steps

- Manual verification with real OpenAI workspace
- Consider if Naive mode needs similar fix
- Monitor for user feedback confirming fix works

## Lessons/Insights

- When creating method variants (\_with_vector_storage), critical logic like source_chunk_ids collection must be explicitly copied
- Semantic search for mixed vector types (entities+chunks) often returns wrong type in top-K
- ID-based filtering is more reliable than type filtering after semantic search
