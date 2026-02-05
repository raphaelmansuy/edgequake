# Iteration 05: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Analysis: Documentation Priorities

### High-Value Documentation Targets

1. **Span Merging Magic Numbers**
   - `0.5` font size tolerance
   - `0.3 * font_size` y-tolerance
   - `0.25 * font_size` space threshold
   - `0.3 * avg_char_width` overlap tolerance

2. **Font Style Data Flow Diagram**
   - Show how data flows from PDFium to markdown output
   - Include fallback paths for legacy backends

### Implementation Plan

Add comprehensive WHY documentation to `pymupdf_structs.rs`:

```rust
/// ## Font Style Detection Data Flow
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────────────┐
/// │                    Font Style Pipeline                                   │
/// ├─────────────────────────────────────────────────────────────────────────┤
/// │  PDFium Backend (pdfium.rs)                                             │
/// │  ├─ font_weight() → is_bold: bool                                       │
/// │  ├─ font_is_italic() → is_italic: bool                                  │
/// │  └─ font_is_fixed_pitch() → is_monospace: bool                          │
/// │           ↓                                                             │
/// │  RawChar (elements.rs)                                                  │
/// │  ├─ is_bold: bool                                                       │
/// │  ├─ is_italic: bool                                                     │
/// │  └─ is_monospace: bool                                                  │
/// │           ↓                                                             │
/// │  Span (pymupdf_structs.rs)                                              │
/// │  ├─ font_is_bold: Option<bool>    ← Copied from first char              │
/// │  ├─ font_is_italic: Option<bool>  ← Copied from first char              │
/// │  └─ font_is_monospace: Option<bool> ← Copied from first char            │
/// │           ↓                                                             │
/// │  Markdown Rendering (pymupdf_renderer.rs)                               │
/// │  ├─ is_bold() → **text**                                                │
/// │  ├─ is_italic() → _text_                                                │
/// │  └─ is_monospace() → `text`                                             │
/// └─────────────────────────────────────────────────────────────────────────┘
/// ```
```
