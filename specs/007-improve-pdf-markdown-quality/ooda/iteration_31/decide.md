# IT31 — Decide: Remove lopdf Legacy Code

## Decision
Remove ALL lopdf-dependent code to establish a single, clean extraction pipeline.

## Plan
1. Delete 13 backend source files (lopdf modules)
2. Delete 10 debug binary files in `src/bin/`
3. Delete 7 example files that depend on lopdf
4. Update `Cargo.toml`: remove lopdf dependency and feature flag
5. Update `backend/mod.rs`: remove lopdf module declarations
6. Update `lib.rs`: remove image_extraction module and lopdf re-exports
7. Update `extractor.rs`: simplify backend selection
8. Update `processors/mod.rs`: remove image_processor module
9. Verify: `cargo test --lib` passes, `cargo clippy` clean

## Impact
- Remove ~13,602 lines of dead code
- Simplify Cargo.toml features from 3 backends to 1
- No regression: lopdf was never used when pdfium was available
