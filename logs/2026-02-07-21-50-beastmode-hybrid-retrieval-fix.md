# Task Log: RAG Hybrid Retrieval Fix

**Date:** 2026-02-07 21:50  
**Session:** beastmode-hybrid-retrieval-fix

## Actions

- Switched LLM provider from Ollama/Gemma to OpenAI gpt-4o-mini
- Fixed `make backend-bg` to auto-detect OpenAI provider (previously always set OLLAMA_HOST)
- Ran full 100-question hybrid eval (pre-fix): Overall=0.529, Recall=38.4%
- Investigated root cause: hybrid mode only retrieved chunks via entity→source_chunk_id mapping
- Ran full naive eval: Overall=0.715, Recall=77.0%
- Fixed hybrid mode to include naive chunk retrieval via `tokio::join!` parallel execution
- Fixed metrics.py source matching to check snippet content when file_path is empty
- Ran full 100-question hybrid eval (post-fix): Overall=0.729, Recall=78.6%, Correctness=89.3%
- Investigated remaining 3 problem documents (TR01, VH13, IFRS-01): chunks exist but rank below top-10
- Committed: `387d73b3` (hybrid fix + Makefile), `fea7c551` (API + UI improvements)

## Decisions

- Naive chunks form the base of hybrid results (ensures high recall)
- Entity-based chunks from local+global are merged on top with dedup
- Entities/relationships from graph are preserved for LLM context enrichment
- 3 remaining problem docs (0% recall) are vector ranking issues, not missing data

## Results Comparison

| Metric      | Old Hybrid | Naive | Fixed Hybrid |
| ----------- | ---------- | ----- | ------------ |
| Overall     | 0.529      | 0.715 | **0.729**    |
| Recall      | 38.4%      | 77.0% | **78.6%**    |
| Correctness | 63.6%      | 86.0% | **89.3%**    |
| Precision   | 76.2%      | —     | **94.9%**    |

## Next Steps

- Consider increasing max_chunks from 10 to 15 for marginal recall improvement
- Prefix chunks with document title before embedding for better source attribution
- Fix FICHE-RET-03 / TA03-02 server disconnect errors (2 failures)
- Re-evaluate with larger chunk context for problem categories (treasury_payments, fiscal_admin)

## Lessons/Insights

- Entity-based retrieval is complementary but cannot be the sole chunk source
- Naive vector search provides the recall foundation; entities add reasoning context
- 10.7% of answers are correct but cite alternative (related) source documents
