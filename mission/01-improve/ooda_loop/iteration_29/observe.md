# OODA-29 Observe/Orient/Decide

## Observe
- `keywords/intent.rs` (224 lines, 7 tests) — QueryIntent heuristic classifier. Missing: from_str_loose, Display, recommended_mode for all variants, empty query, Default trait
- `chunk_retrieval.rs` (344 lines, 5 tests) — Missing: merge_chunks edge cases (empty lists, single list, all duplicates)

## Orient
Pure functions with high signal for edge case testing. No WHY comments on why heuristic classification exists (fallback for when LLM is unavailable).

## Decide
1. Add WHY comment to intent.rs about heuristic fallback
2. Add WHY comment to chunk_retrieval.rs about merge deduplication strategy
3. Add ~12 edge case tests across both files
