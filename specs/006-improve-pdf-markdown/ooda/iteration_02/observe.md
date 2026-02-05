# Iteration 02: Observe

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Territory Mapping - Font Style Rendering

### Focus: Verify font style flags propagate to Markdown output

Tracing the actual font style flow to identify where styles might be lost.

### 1. RawChar Creation (pdfium.rs:164-178)

```rust
// PDFium provides accurate font style from font descriptor
let is_italic = char_obj.font_is_italic();
let is_bold = char_obj.font_weight().is_some_and(|w| {
    matches!(w, PdfFontWeight::Weight700Bold | PdfFontWeight::Weight800 | ...)
});

// RawChar carries these flags
RawChar {
    char: c,
    is_bold,      // ✓ From font_weight()
    is_italic,    // ✓ From font_is_italic()
    ...
}
```

### 2. RawChar → Span Conversion (pymupdf_structs.rs:131-143)

```rust
// Span::append() copies style from first character
pub fn append(&mut self, ch: &RawChar) {
    if self.text.is_empty() {
        // Copy font flags from first character
        self.font_is_bold = Some(ch.is_bold);     // ✓ Preserved
        self.font_is_italic = Some(ch.is_italic); // ✓ Preserved
    }
    ...
}
```

### 3. Span Style Accessors (pymupdf_structs.rs:164-190)

```rust
pub fn is_bold(&self) -> bool {
    // Prefer font descriptor flag from PDFium (most accurate)
    if let Some(is_bold) = self.font_is_bold {
        return is_bold;  // ✓ Returns PDFium flag
    }
    // Fallback to font name matching
    ...
}
```

### 4. Style Detection (pymupdf_renderer.rs:296-309)

```rust
fn get_style_type(span: &Span) -> StyleType {
    if span.is_bold() && span.is_italic() {
        StyleType::BoldItalic  // ✓ Both flags checked
    } else if span.is_bold() {
        StyleType::Bold        // ✓ Bold checked
    } else if span.is_italic() {
        StyleType::Italic      // ✓ Italic checked
    } else if span.is_monospace() {
        StyleType::Code
    } else {
        StyleType::Plain
    }
}
```

### 5. Style Application (pymupdf_renderer.rs:316-335)

```rust
fn apply_style(text: &str, style: StyleType) -> String {
    let styled = match style {
        StyleType::BoldItalic => format!("**_{}_**", trimmed),  // ✓
        StyleType::Bold => format!("**{}**", trimmed),         // ✓
        StyleType::Italic => format!("_{}_", trimmed),         // ✓
        StyleType::Code => format!("`{}`", trimmed),           // ✓
        _ => trimmed.to_string(),
    };
    ...
}
```

## Findings

### ✅ Font Style Chain is WORKING

The entire chain from PDFium → RawChar → Span → Markdown is correctly implemented:

1. **PDFium extraction**: `font_is_italic()` and `font_weight()` provide accurate flags
2. **Span creation**: `append()` copies `is_bold` and `is_italic` from first char
3. **Style detection**: `get_style_type()` checks both flags
4. **Markdown output**: `apply_style()` wraps text correctly

### ⚠️ Potential Issues Identified

1. **First-char assumption**: If the first character of a span has wrong style,
   the entire span inherits that style. This could be problematic if a span
   starts with a space or punctuation that has a default style.

2. **Font name fallback**: If `font_is_bold` is `None`, the fallback uses
   pattern matching which is less reliable than PDFium flags.

3. **Can_append style check missing**: The `can_append()` method doesn't check
   if `is_bold` or `is_italic` match between characters. This could merge
   bold and non-bold text into the same span.

### Next Steps

Focus iteration 02 on fixing the `can_append()` style check issue.
