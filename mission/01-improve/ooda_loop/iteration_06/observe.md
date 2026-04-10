# Observe — Iteration 06

Date: 2026-04-10  
Mission file re-read: `mission/01-improve.md`

## Findings

`InMemoryKeywordCache` in `crates/edgequake-query/src/keywords/cache.rs` uses `RwLock::read().unwrap()` and `RwLock::write().unwrap()` across 10+ call sites. If any thread panics while holding the lock, all subsequent accesses will panic due to lock poisoning.

For a **cache** (reconstructible data), panicking on poison is incorrect — the cache should degrade gracefully.

## Risk

- **Severity**: Medium (poison cascade can take down the query engine)
- **Likelihood**: Low (requires a prior panic while holding the lock)
- **Fix**: Replace `.unwrap()` with `.unwrap_or_else(|e| e.into_inner())` to recover from poisoned locks
