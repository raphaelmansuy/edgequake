# OODA-26 Act: WHY Comments + Edge Case Tests for Query Crate

## Changes Made

### WHY Comments Added (5 files)

1. **context.rs** — WHY: Separate collections per retrieval type (token budgets, rendering order, provenance), WHY: Incremental token counting (O(1) vs O(n)), WHY: Section order in `to_context_string` (BR0102), ASCII diagram of QueryContext.
2. **context_filter.rs** — WHY: Strict chunks vs. lenient entities/relationships (cross-document knowledge preservation), ASCII diagram of filtering strategy.
3. **error.rs** — WHY: Separate error variants for user vs. system failures, `#[from]` rationale.
4. **lib.rs** — WHY: Flat re-export facade (reduces import churn on refactors).
5. **vector_filter.rs** — WHY: Type stored as string in JSON metadata (storage independence, no enum versioning, debug-friendly).

### Edge Case Tests Added (+21 tests)

**context.rs** (+11):
- `test_empty_context_to_string_is_empty`
- `test_empty_context_is_empty`
- `test_token_count_accumulates_on_add_chunk`
- `test_entity_degree_display_in_context_string`
- `test_relationship_with_description_rendering`
- `test_relationship_without_description_rendering`
- `test_chunk_score_formatting_in_context_string`
- `test_chunk_reference_ids_are_one_based`
- `test_retrieved_chunk_token_count_empty_content`
- `test_retrieved_chunk_builder_chain`
- `test_retrieved_context_default`

**error.rs** (+6):
- `test_invalid_query_display`
- `test_no_results_display`
- `test_context_limit_exceeded_display`
- `test_timeout_display`
- `test_internal_display`
- `test_config_error_display`

**vector_filter.rs** (+4):
- `test_vector_type_as_str_values`
- `test_filter_non_string_type_value`
- `test_get_typed_vectors_limit_zero`
- `test_get_typed_vectors_limit_exceeds_count`

## Evidence

- Tests: 1270 → 1291 (+21)
- Clippy: 0 warnings
