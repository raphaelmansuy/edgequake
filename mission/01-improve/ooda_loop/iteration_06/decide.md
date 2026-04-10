# Decide — Iteration 06

Date: 2026-04-10  
Mission file re-read: `mission/01-improve.md`

## Scope

Replace all `RwLock::read().unwrap()` / `RwLock::write().unwrap()` in `cache.rs` with poison-recovering variants. This applies to both InMemoryKeywordCache and PostgresKeywordCache stats.

## Verification

- `cargo test -p edgequake-query --lib`
- `cargo clippy -p edgequake-query`
