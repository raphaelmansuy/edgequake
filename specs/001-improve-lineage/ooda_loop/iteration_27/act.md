# Implementation - Iteration 27

## Changes Made

1. **File**: `edgequake-api/src/handlers/lineage.rs`
   - Lines: ~385-420 — Replaced simple `doc_map.into_iter().map()` with loop that:
     - Resolves document name from `{doc_id}-metadata` via `cached_kv_get()`
     - Resolves chunk `start_line`/`end_line` from `{chunk_id}` KV via `cached_kv_get()`
   - Fixed `u64` → `usize` type conversion for chunk position fields

## Verification
- `cargo build -p edgequake-api`: ✅ Compiles cleanly
- `cargo test -p edgequake-api --lib`: ✅ 459 passed, 0 failed
