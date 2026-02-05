# Iteration 03: Observe

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Focus: Monospace/Code Detection

Looking at the font style chain, we have bold and italic working. Now let's verify monospace (code) detection.

### Code Detection in PDFium Pipeline

**pdfium.rs** - Currently does NOT detect monospace:

```rust
// No monospace detection in extract_chars_from_page()
let is_italic = char_obj.font_is_italic();
let is_bold = char_obj.font_weight().is_some_and(|w| ...);
// Missing: is_monospace
```

**RawChar** (elements.rs) - No monospace field:

```rust
pub struct RawChar {
    pub is_bold: bool,
    pub is_italic: bool,
    // Missing: is_monospace
}
```

**Span** (pymupdf_structs.rs:209-234) - Has is_monospace() but relies on font name:

```rust
pub fn is_monospace(&self) -> bool {
    self.font_name
        .as_ref()
        .map(|n| {
            let lower = n.to_lowercase();
            lower.contains("mono")
                || lower.contains("courier")
                || lower.contains("consolas")
                || lower.contains("menlo")
                || lower.contains("source code")
                || lower.contains("fira code")
                || lower.contains("jetbrains")
                || lower.contains("inconsolata")
        })
        .unwrap_or(false)
}
```

### Issue Identified

1. PDFium provides `font_is_fixed_pitch()` API that can detect monospace fonts
2. We're not using this accurate API
3. Instead, we rely on font name patterns which can miss fonts like "Monaco" or "Input"

### PDFium API Available

```rust
// pdfium-render provides:
char_obj.font_name() -> String
// But does pdfium-render expose is_fixed_pitch?
```

Need to check pdfium-render documentation for monospace detection capability.
