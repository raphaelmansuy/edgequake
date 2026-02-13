# Analysis - Iteration 23

## Gaps Identified
1. No caching for lineage/metadata KV lookups — repeated requests hit storage
2. Lineage data is immutable after processing but fetched fresh each time
3. Dashboard polling and UI navigation create repeated identical queries

## Possible Solutions

### Solution A: In-memory TTL cache (reuse workspace stats pattern)
- Pros: Proven pattern, sub-millisecond cache hits, bounded memory, simple invalidation
- Cons: Cache invalidation on reprocessing required
- Risk: Low — follows established codebase pattern

### Solution B: HTTP-level caching (Cache-Control / ETag headers)
- Pros: Browser/CDN caching, no server-side state
- Cons: Complex for dynamic data, doesn't help server-side N+1
- Risk: Medium — requires careful header management

## Recommendation
Solution A — In-memory TTL cache with `lazy_static!` global. 120s TTL (lineage changes less often than workspace stats). Max 500 entries with eviction. Includes `invalidate_lineage_cache()` for reprocessing scenarios.
