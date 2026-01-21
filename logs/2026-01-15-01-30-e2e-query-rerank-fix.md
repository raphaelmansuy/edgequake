# Task Log: E2E Query Rerank Score Fix

**Date**: 2026-01-15 01:30 UTC  
**Mode**: beastmode  
**Duration**: ~45 minutes

## Actions

- Investigated chunks not appearing in query responses despite vector retrieval
- Added debug logging to trace chunk flow through reranking pipeline
- Identified `min_rerank_score: 0.3` as the root cause (BM25 score 0.287 was filtered out)
- Fixed by lowering `min_rerank_score` from 0.3 to 0.1
- Cleaned up debug logging after fix verification
- Tested new workspace creation + document upload + query flow
- Ran full test suite (2,456 tests pass)
- Committed fix: `cfc6929`

## Decisions

- Chose 0.1 for `min_rerank_score` to match `min_score` setting for consistency
- Added WHY comment explaining the rationale for future maintainers

## Next Steps

- Monitor query quality in production to ensure 0.1 threshold is appropriate
- Consider making `min_rerank_score` configurable per workspace
- Graph storage workspace isolation needs audit (currently uses shared storage)

## Lessons/Insights

- BM25 scores can be surprisingly low for short documents/simple queries
- Always trace the full data flow when debugging "missing" results
- The fix was simple but diagnosis required understanding the reranking pipeline
