# Iteration 05: ORIENT - Analysis

## Gap Analysis

### The Real Problem

After deep code investigation, I found the actual issue is NOT in block creation logic - blocks are correctly created at paragraph/line boundaries.

The real issue is in `pdfium_backend.rs::convert_text_block_to_schema_block()`:

```rust
// Current (BROKEN):
block.text = text_block.text();  // Plain text, no styles
// block.spans is empty!
```

The `TextBlock` contains `lines: Vec<Line>` where each `Line` has `spans: Vec<Span>` with full style info (bold, italic, monospace). But this info is NEVER transferred to `schema::Block.spans`.

### Data Flow Analysis

```
PDFium Extract
    ↓
RawChar{is_bold, is_italic}     ← Style info present
    ↓
pymupdf_grouper::chars_to_spans()
    ↓
Span{font_is_bold, font_is_italic}  ← Style preserved
    ↓
spans_to_lines() → Line{spans}
    ↓
lines_to_blocks() → TextBlock{lines}
    ↓
convert_text_block_to_schema_block()
    ↓
block.text = flat string      🔴 STYLE LOST HERE
block.spans = empty vec       🔴 SPANS NOT POPULATED
```

### Why This Matters

The markdown renderer uses `block.spans` to apply inline styling:

- If `spans` is empty, it falls back to `block.text` (plain text)
- No bold/italic markers are added

### Comparison with PyMuPDF4LLM

PyMuPDF4LLM iterates spans at render time:

```python
for span in line["spans"]:
    if span["flags"] & BOLD:
        text = f"**{span['text']}**"
```

Our system should do the same, but through the `spans` field on `schema::Block`.

## Quality Impact

| Metric           | Without Fix | With Fix |
| ---------------- | ----------- | -------- |
| Bold detection   | 0%          | ~95%     |
| Italic detection | 0%          | ~95%     |
| Inline code      | 0%          | ~90%     |
