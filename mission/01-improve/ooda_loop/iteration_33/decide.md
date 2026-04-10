# OODA-33 — Decide

Add test module to `text_utils.rs` with ~15 tests:
- calculate_line_numbers: single line, multi-line, empty string, offset at boundary
- estimate_tokens: empty, short, long
- split_into_sentences: basic, abbreviations not split, no ending punctuation
- floor_char_boundary: ASCII, multibyte, beyond end
- ceil_char_boundary: ASCII, multibyte, beyond end
- take_overlap_sentences: empty buffer, exact fit

Add tests to `types.rs`:
- ChunkerConfig::default all fields
- TextChunk::new token count, set_line_numbers
