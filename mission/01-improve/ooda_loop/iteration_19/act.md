# OODA-19 Act: Edge Case Tests for Chunker Text Utils and Lineage

## Changes Made

### chunker/mod.rs (+18 tests)
- `test_estimate_tokens_empty_string` — empty input returns 0
- `test_estimate_tokens_single_char` — single byte ceil to 1 token
- `test_estimate_tokens_multibyte` — multi-byte chars inflate token count
- `test_calculate_line_numbers_offsets_beyond_text` — clamped to text length
- `test_calculate_line_numbers_start_equals_end` — zero-width span
- `test_calculate_line_numbers_empty_text` — empty text returns (1,1)
- `test_floor_char_boundary_at_zero` / `_beyond_length`
- `test_ceil_char_boundary_at_length` / `_beyond_length` / `_at_zero`
- `test_split_into_sentences_empty` / `_only_abbreviations` / `_trailing_text_no_period`
- `test_chunk_whitespace_only` / `_single_newline`
- `test_character_chunker_construction` / `test_default_chunker_config_values`

### lineage.rs (+10 tests)
- `test_entity_lineage_no_descriptions` — None when empty
- `test_entity_lineage_multiple_descriptions_returns_last`
- `test_description_version_origins` — extraction/merge/summary
- `test_document_lineage_get_nonexistent_entity`
- `test_document_lineage_chunks_for_nonexistent_entity`
- `test_relationship_lineage_construction`
- `test_entity_source_multiple_chunks`
- `test_extraction_metadata_defaults`
- `test_lineage_builder_record_relationship`
- `test_chunk_lineage_with_lines_and_offsets`

### Cleanup
- Removed unused imports: TenantPlan, MembershipRole, HashMap, parse_plan

## Metrics
- Tests: 1168 → 1196 (+28)
- Clippy warnings: 0
- Commit: db60b95d
