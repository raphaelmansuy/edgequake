# OODA Iteration 27 – Act

## Changes Applied

### `src/processors/text_cleanup.rs` – `is_garbled()` method

1. Added `"&"` to `valid_short_words` array with WHY comment
2. Added section number pattern skip after the author initials skip:

```rust
if chars.len() == 2 && chars[0].is_ascii_digit() && (chars[1] == ')' || chars[1] == '.') {
    return false;
}
```

## Verification

- **569 tests pass**
- **4 section titles restored**:
  - `0) AI Strategy & Co‑Creation`
  - `1) AI Agent Design & Building`
  - `Search UX & APIs`
  - `Industrialization (4 8+ weeks)`
- Output: 168 lines (8 new content lines, offsetting the 8 removed from IT26 page breaks)
