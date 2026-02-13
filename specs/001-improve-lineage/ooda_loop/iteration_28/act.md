# Action - Iteration 28

## Changes Applied
1. `openapi.rs` — Added 15 lineage DTO schemas + `ExportParams` to `components(schemas())`
2. `lineage.rs` — Added `Serialize`, `ToSchema` derives to `ExportParams`

## Verification
- `cargo build` — clean
- `cargo test -p edgequake-api --lib` — 459 passed, 0 failed
