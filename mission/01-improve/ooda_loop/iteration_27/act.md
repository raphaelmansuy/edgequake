# OODA-27 Act: WHY Comments for Storage/Core/Pipeline/Tasks + 8 Edge Case Tests

## WHY Comments Added (5 files)

1. **storage/error.rs** — WHY: Fine-grained error variants for HTTP mapping + ASCII tree
2. **core/error.rs** — WHY: Two-level hierarchy (Error wraps crate errors, QueryError has retry semantics) + ASCII tree
3. **pipeline/cache.rs** — WHY: Cache key = content_hash + model + prompt_version (no stale hits, model isolation) + ASCII flow
4. **pipeline/validation.rs** — WHY: Blocked extensions as security boundary (defense-in-depth, separate from whitelist)
5. **tasks/queue.rs** — WHY: Arc<Mutex<Receiver>> for multi-consumer (mpsc is single-consumer) + ASCII diagram

## Edge Case Tests (+8)

**cache.rs** (+5): empty inputs cache key, model matters for key, default tokens zero, CacheType variants distinct, empty stats
**queue.rs** (+3): size after send, unbounded not closed, try_receive returns Some when available

## Evidence
- Tests: 1291 → 1299 (+8)
- Clippy: 0 warnings
- Commit: OODA-27
