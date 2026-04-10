# OODA-37 — Observe/Orient/Decide

## Target: `crates/edgequake-pipeline/src/extractor/mod.rs` (458 lines, 6 existing tests)

### Untested edge cases:
1. ExtractedEntity::new defaults (importance=0.5, empty spans, None embedding)
2. with_importance clamping (below 0.0, above 1.0)
3. with_source_chunk_id deduplication
4. add_source_chunk_id deduplication
5. ExtractedRelationship::new defaults (weight=0.5, empty description)
6. with_weight clamping (below 0.0, above 1.0)
7. ExtractionResult::new defaults (empty, 0 tokens, 0 timing)
8. with_token_usage, with_timing
9. extract_json_from_response (private fn, testable from within module)
