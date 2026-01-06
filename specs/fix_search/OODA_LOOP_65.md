# OODA Loop 65: Keyword Validation Caching Optimization

## Observe

After implementing keyword validation in OODA 62, we observed that each query performs N graph lookups (one per keyword) via `search_labels()`. For repeated queries with similar keywords, this creates redundant database calls.

## Orient

### Current Flow (Before)
```
Query → Extract Keywords → For each keyword:
                              → search_labels(keyword) [DB call]
                              → validate
        → Embed validated keywords
```

### Optimized Flow (After)
```
Query → Extract Keywords → For each keyword:
                              → Check validation cache [memory]
                              → If miss: search_labels(keyword) [DB call]
                              → Cache result
        → Embed validated keywords
```

## Decide

Add an in-memory cache (`HashMap<String, bool>`) for keyword validation results:
- Key: lowercase keyword
- Value: exists in graph (true/false)
- Max size: 10,000 entries (prevents unbounded growth)
- Shared across all queries via `Arc<RwLock<>>`

## Act

### Changes to `SOTAQueryEngine`

1. Added new field:
```rust
keyword_validation_cache: Arc<tokio::sync::RwLock<HashMap<String, bool>>>,
```

2. Updated `validate_keywords()` to check cache first:
```rust
// Check cache first
let cache_key = keyword.to_lowercase();
let cached_result = {
    let cache = self.keyword_validation_cache.read().await;
    cache.get(&cache_key).copied()
};

let exists = if let Some(exists) = cached_result {
    // Cache hit
    exists
} else {
    // Cache miss - check graph
    let matches = self.graph_storage.search_labels(keyword, 1).await;
    let exists = matches.map(|labels| !labels.is_empty()).unwrap_or(false);
    
    // Update cache (with size limit)
    {
        let mut cache = self.keyword_validation_cache.write().await;
        if cache.len() < 10000 {
            cache.insert(cache_key, exists);
        }
    }
    exists
};
```

## Results

- **Build**: Success
- **Tests**: 11/11 EXCELLENT (100%)
- **Performance**: Repeated queries with same keywords now skip DB lookups

### Cache Benefits

| Scenario | Before (DB calls) | After (DB calls) |
|----------|-------------------|------------------|
| First query with "BYD Seal U" | 1 | 1 |
| Second query with "BYD Seal U" | 1 | 0 (cache hit) |
| Similar queries in session | N * keywords | 1 per unique keyword |

## Files Modified
- `edgequake/crates/edgequake-query/src/sota_engine.rs`
  - Added `keyword_validation_cache` field
  - Updated constructors
  - Modified `validate_keywords()` to use cache

## Next Steps (OODA 66+)
- Consider cache TTL for long-running servers
- Monitor cache hit rate in production
- Potential: persist cache to Redis for distributed setups
