# OODA-33 — Act

## Changes Made

### 1. Tests in `chunker/text_utils.rs` (+18 tests, was 0)
- `calculate_line_numbers`: single line, multi-line, empty, beyond-end
- `estimate_tokens`: empty, short, exact multiple
- `split_into_sentences`: basic 3-sentence, abbreviation preserved, no ending
- `floor_char_boundary`: ASCII, multibyte mid-char, beyond end
- `ceil_char_boundary`: ASCII, multibyte mid-char, beyond end
- `take_overlap_sentences`: empty buffer, takes from end

### 2. Tests in `chunker/types.rs` (+4 tests, was 0)
- `ChunkerConfig::default` all fields
- `TextChunk::new` token count, defaults
- `TextChunk::with_line_numbers` all fields
- `set_line_numbers` mutation

## Test Evidence

- **Pipeline chunker**: 68 passed
- **Workspace total**: 1383 passed, 0 failed
