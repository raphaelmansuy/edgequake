# Action - Iteration 29

## Changes Applied
1. `lineage.rs` — Added 5 WHY comments explaining:
   - chunk ID → doc ID extraction (F8 bidirectional chain)
   - entity name normalization (UPPERCASE_WITH_UNDERSCORES convention)
   - KV key prefix scan trade-off
   - content preview truncation (dashboard readability)
   - lineage + metadata combination (F5 single-call goal)
2. `lineage.rs` — Improved 3 error messages:
   - Entity provenance: shows normalized name and explains convention
   - Entity lineage: same improvement
   - Export handler: added "Document may not have been processed yet."

## Verification
- `cargo test -p edgequake-api --lib` — 459 passed, 0 failed
- `cargo clippy -p edgequake-api --lib` — 0 warnings
