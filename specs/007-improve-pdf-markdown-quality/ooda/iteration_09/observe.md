# Iteration 09: OBSERVE - Code Block Detection

## Focus Area

Code blocks are high priority (70→90 target). Need to understand:
1. How code blocks are currently detected
2. What monospace font detection looks like
3. Potential improvements

## Code Inventory

### Code Detection Flow

```
PDF Font Family  ──►  looks_like_code()  ──►  Block Type
     │                      │
     │                      ├─ contains("mono")? → Code
     │                      ├─ contains("courier")? → Code
     │                      ├─ contains("consolas")? → Code
     │                      └─ ... → Text
     │
     └── Applied in CodeBlockDetectionProcessor
```

### Key Files

| File | Purpose |
|------|---------|
| `schema/block.rs` | FontStyle::looks_like_code() |
| `processors/structure_detection.rs` | CodeBlockDetectionProcessor |
| `renderers/markdown.rs` | render_code(), render_spans_styled() |

### Current Font Detection

From `schema/block.rs:84-94`:

```rust
pub fn looks_like_code(&self) -> bool {
    lower.contains("mono")
        || lower.contains("courier")
        || lower.contains("consolas")
        || lower.contains("source code")
}
```

**Only 4 font patterns!** Many monospace fonts would be missed.

## Problem Identification

### Missing Fonts (from Wikipedia "List of monospaced typefaces")

Programming fonts NOT detected:
- JetBrains Mono (contains "jetbrains" not "mono" in some variants)
- Fira Code, Fira Mono
- Inconsolata
- Hack
- Iosevka
- Monaco (Mac default - "monaco" doesn't contain "mono")

System fonts NOT detected:
- Menlo
- SF Mono
- Lucida Console
- DejaVu Sans Mono
- Liberation Mono

Classic fonts NOT detected:
- Letter Gothic
- Prestige Elite
- Fixedsys
- OCR-A, OCR-B

### Test Coverage

15 code tests passing, including:
- `test_font_style_code_detection` - Only tests Courier, Consolas
- `test_code_block_detection`
- `test_code_block_rendering`

## Opportunity

Extend `looks_like_code()` with comprehensive font pattern list to catch
all common monospace fonts used in PDFs.

