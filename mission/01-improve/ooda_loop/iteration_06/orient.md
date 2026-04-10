# Orient — Iteration 06

Date: 2026-04-10  
Mission file re-read: `mission/01-improve.md`

## First Principles

A cache holds reconstructible data. If the lock is poisoned:
1. The data may be partially written, but cache-miss is always safe (re-fetch from source)
2. Panic cascading from poison kills the entire query engine
3. Recovering stale cache data is better than crashing

## Approach

Add a private helper method `read_cache()` and `write_cache()` that use `unwrap_or_else(|e| e.into_inner())` to silently recover from poison. Add WHY comment explaining the rationale.
