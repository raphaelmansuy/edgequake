# OODA-33 — Observe

## Target: Pipeline Chunker text_utils.rs + types.rs (0 tests)

### Pure Functions in text_utils.rs
1. `calculate_line_numbers(full_text, start, end)` — line number calculation from char offsets
2. `estimate_tokens(text)` — len/4 ceil heuristic
3. `split_into_sentences(text)` — abbreviation-aware sentence splitting
4. `floor_char_boundary(s, index)` — UTF-8 safe floor
5. `ceil_char_boundary(s, index)` — UTF-8 safe ceil
6. `take_overlap_sentences(buffer, target)` — overlap from sentence buffer
7. `split_text_internal(text, target, overlap, min, seps)` — core chunking
8. `find_split_point_internal(text, target, seps)` — separator-aware split

### Pure Functions in types.rs
9. `ChunkerConfig::default()` — chunk_size=1200, overlap=100, min=100
10. `TextChunk::new()` — token count via estimate_tokens
11. `TextChunk::with_line_numbers()` — includes line numbers
12. `TextChunk::set_line_numbers()` — mutation

All functions are pub(super) except calculate_line_numbers. Tests must be in same module.
