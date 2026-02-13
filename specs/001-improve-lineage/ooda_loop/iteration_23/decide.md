# Decision - Iteration 23

## Changes to Make
1. `lineage.rs` — Add `CachedLineage`, `LineageCache`, `LINEAGE_KV_CACHE` global (120s TTL, 500 max entries)
2. `lineage.rs` — Add `cached_kv_get()` function that checks cache before KV storage
3. `lineage.rs` — Add `invalidate_lineage_cache()` for use after document reprocessing
4. `lineage.rs` — Replace 4 direct `kv_storage.get_by_id()` calls with `cached_kv_get()`
5. `lineage.rs` — Add 3 cache-specific unit tests

## Priority
1. Cache infrastructure (foundational)
2. Replace KV calls (performance gain)
3. Invalidation function (correctness)
4. Tests (validation)

## Expected Outcome
- Cache hits serve lineage in <1ms vs ~15ms uncached
- Cache auto-evicts expired entries and caps at 500 to prevent memory leaks
- `invalidate_lineage_cache()` available for document reprocessing
- All 459 API tests pass
