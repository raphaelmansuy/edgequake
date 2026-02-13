# Implementation - Iteration 01

## Changes Made

1. File: `edgequake/crates/edgequake-core/src/types/chunk.rs`
   - Lines: 45-60 (Chunk struct)
   - Change: Added `start_line`, `end_line`, `start_offset`, `end_offset` as `Option<usize>` fields with `serde(default)` for backward compat
   - Added `with_position()` builder method for setting position metadata
   - Commit: `3bc77469`

## Tests Added/Updated

- `test_chunk_position_default_none` — verifies new fields default to None (passing)
- `test_chunk_with_position` — verifies builder sets all 4 position fields (passing)
- `test_chunk_position_serialization_roundtrip` — verifies JSON round-trip for position data (passing)
- `test_chunk_backward_compat_deserialization` — verifies old JSON without position fields still deserializes (passing)

## Verification

- `cargo test -p edgequake-core --lib -- chunk`: ✅ 7 passed
- `cargo test --workspace --lib`: ✅ 1688 passed, 0 failed
- `cargo clippy`: ✅ (no new warnings)
