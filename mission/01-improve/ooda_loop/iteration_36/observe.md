# OODA-36 — Observe

## Target: `crates/edgequake-pipeline/src/merger/mod.rs`

### Pure functions available:
1. `normalize_entity_name` — already 1 test with 4 cases, but missing: empty string, Unicode, numbers, single char
2. `merge_descriptions` — already 1 test, missing: contained substring skip, max length truncation
3. `truncate_description` — already 1 test, missing: exact boundary, no sentence end
4. `MergerConfig::default` — 0 tests, 5 fields
5. `MergeStats::default` — partial test, missing: default values all zero

Also: `crates/edgequake-pipeline/src/prompts/normalizer.rs` — the normalize function used by parsers
