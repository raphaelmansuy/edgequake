# Decision - Iteration 27

## Changes to Make
1. `lineage.rs` — Replace `doc_map.into_iter().map()` with cached KV lookups for document names and chunk positions
2. Use `cached_kv_get()` (OODA-23 cache) to avoid N+1 penalty

## Expected Outcome
- Entity provenance response includes document names and chunk line ranges
- Cache prevents I/O penalty for repeated provenance queries
