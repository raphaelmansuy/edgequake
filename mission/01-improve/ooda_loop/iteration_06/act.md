# Act — Iteration 06

Date: 2026-04-10  
Commit: `eba796e4`

Replaced 14 `RwLock::read/write().unwrap()` → `.unwrap_or_else(|e| e.into_inner())` in `crates/edgequake-query/src/keywords/cache.rs`. Added WHY doc-comment explaining poison-recovery rationale for cache data.

Verification: `cargo test -p edgequake-query --lib` → 92 passed, 0 failed.
