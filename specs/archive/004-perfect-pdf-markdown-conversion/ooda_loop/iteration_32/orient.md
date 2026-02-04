# OODA-32 ORIENT: Micro-Test Strategy

## Analysis

### Test Design Philosophy

```
┌────────────────────────────────────────────────────────────────┐
│                   Micro-Test Design Goals                       │
├────────────────────────────────────────────────────────────────┤
│ 1. SPEED: <0.1s per test file (no compilation overhead)        │
│ 2. ISOLATION: Test one feature per file                        │
│ 3. DETERMINISM: Use embedded bytes, no file I/O                │
│ 4. CLARITY: One assertion per test, clear failure messages     │
└────────────────────────────────────────────────────────────────┘
```

### PDF Selection Strategy

| Test            | PDF File                           | Size  | Why                |
| --------------- | ---------------------------------- | ----- | ------------------ |
| micro_text      | 001_simple_text.pdf                | 1.7KB | Minimal plain text |
| micro_tables    | 004_simple_table_2x3.pdf           | 1.9KB | Simple 2x3 table   |
| micro_columns   | 003_two_columns.pdf                | 2.0KB | Clear two-column   |
| micro_fonts     | 023_incomplete_unicode_mapping.pdf | 1.6KB | Font edge case     |
| micro_structure | legacy/002_headers_and_lists.pdf   | 1.9KB | Headers + lists    |

### First Principles: Why include_bytes!

1. **No I/O latency**: Bytes are in binary at compile time
2. **No file system access**: No path resolution overhead
3. **Reproducible**: Same bytes on every test run
4. **Portable**: No path issues across platforms

### Test Structure

```rust
// Template for each micro-test
const PDF_BYTES: &[u8] = include_bytes!("../test-data/xxx.pdf");

#[test]
fn test_specific_feature() {
    let markdown = extract_markdown(PDF_BYTES);
    assert!(condition, "Clear failure message");
}
```

### Risk Assessment

| Risk                            | Mitigation                                |
| ------------------------------- | ----------------------------------------- |
| PDF too complex for quick parse | Use smallest PDFs (1-2KB)                 |
| Test failures unclear           | Single assertion with descriptive message |
| Compile time increases          | Each test file is independent             |
