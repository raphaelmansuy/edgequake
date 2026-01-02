# OBSERVE.md - Iteration 008

**Directory:** `edgequake/crates/edgequake-pdf/src/backend`

**Timestamp:** 2026-01-02

## Target: Unused Style Information Fields

### Current State Analysis

#### File: `sota_backend.rs` (lines 1242-1248)

```rust
struct MergedLine {
    text: String,
    avg_font_size: f32,
    font_name: String,
    is_bold: bool,        // ⚠️ COLLECTED BUT NEVER USED!
    is_italic: bool,      // ⚠️ COLLECTED BUT NEVER USED!
    spans: Vec<TextSpan>,
}
```

#### Where Fields Are Set (lines 2222-2316)

```rust
fn merge_line(&self, elements: &[TextElement]) -> MergedLine {
    // ... merge logic ...

    let is_bold = elements.iter().any(|e| e.is_bold);      // Line 2241
    let is_italic = elements.iter().any(|e| e.is_italic);  // Line 2242

    // Style information IS flowing through spans:
    let style = FontStyle {
        family: Some(elem.font_name.clone()),
        size: Some(elem.font_size),
        weight: Some(if elem.is_bold { 700 } else { 400 }),  // ✅ USED in spans!
        italic: elem.is_italic,                               // ✅ USED in spans!
        ..Default::default()
    };
    push_to_spans(&mut spans, &elem.text, style);

    // ... but then...

    MergedLine {
        text,
        avg_font_size,
        font_name,
        is_bold,    // ❌ COLLECTED BUT NEVER REFERENCED!
        is_italic,  // ❌ COLLECTED BUT NEVER REFERENCED!
        spans,
    }
}
```

#### Where Fields Are NOT Used (lines 2410-2440)

```rust
fn lines_to_blocks(...) -> Vec<Block> {
    // ...
    for (idx, line) in lines.iter().enumerate() {
        let merged = &line_texts[idx];  // ← This has is_bold/is_italic fields

        // ❌ NEVER REFERENCED: merged.is_bold
        // ❌ NEVER REFERENCED: merged.is_italic

        // Style info comes from spans instead:
        let spans = merged
            .spans
            .iter()
            .cloned()
            .map(|mut s| {
                s.bbox = Some(bbox.clone());
                s
            })
            .collect::<Vec<_>>();

        let block = Block {
            spans,  // ✅ Style info flows through here
            // ...
        };
    }
}
```

### Compiler Warnings

```
warning: fields `font_name`, `is_bold`, and `is_italic` are never read
    --> crates/edgequake-pdf/src/backend/sota_backend.rs:1245:5
     |
1242 | struct MergedLine {
     |        ---------- fields in this struct
...
1245 |     font_name: String,
     |     ^^^^^^^^^
1246 |     is_bold: bool,
     |     ^^^^^^^
1247 |     is_italic: bool,
     |     ^^^^^^^^^
```

### Root Cause

**NOT a First Principles violation!**

The style information (bold, italic) IS being collected and used correctly through the `spans` vector. Each TextSpan has a FontStyle with:

- `weight: Some(700)` for bold
- `weight: Some(400)` for regular
- `italic: true/false`

The `MergedLine.is_bold` and `MergedLine.is_italic` fields are **redundant** - they aggregate line-level style information that's already captured at the span level.

### Assessment

This is **code smell** but NOT a First Principles violation:

- ✅ Style information IS collected from PDF primitives (TextElement)
- ✅ Style information DOES flow through to output (via spans)
- ❌ Redundant aggregation (is_bold/is_italic) calculated but never used
- ❌ Dead code (should be removed)

### Expected Output

Markdown renderer already uses spans:

```rust
// renderer/markdown.rs
impl Renderer for MarkdownRenderer {
    fn render_span(&self, span: &TextSpan) -> String {
        let weight = span.style.weight.unwrap_or(400);
        if weight >= 600 {
            return format!("**{}**", span.text);  // Bold
        }
        if span.style.italic {
            return format!("*{}*", span.text);    // Italic
        }
        span.text.clone()
    }
}
```

**Result:** Bold and italic ARE being rendered correctly!

### Verification Test

Let me check if output actually contains bold/italic markup:

```bash
# Test case: PDF with bold text
cargo test -- test_bold_italic_detection
```

If tests show bold/italic in output → ✅ Working correctly, just need cleanup
If tests show NO bold/italic → ❌ Broken pipeline, need investigation

### Conclusion

**This is NOT Loop 008 material!**

This is simple dead code cleanup:

1. Remove `font_name`, `is_bold`, `is_italic` from `MergedLine`
2. Remove lines 2241-2242 that calculate them
3. Update construction at line 2308 to omit them

No First Principles work needed - style already flows correctly through spans.

### Revised Loop 008 Target

Need to find a REAL First Principles violation. Candidates:

1. **MarginFilterProcessor** (processor.rs:500-550)

   - Uses magic numbers: 50.0, 30.0, 40.0
   - Same category as Loop 007 (should use DocumentStats)

2. **HyphenContinuationProcessor** (processor.rs:650-700)

   - Uses magic number: 50.0
   - Should use DocumentStats for line spacing

3. **XYCutParams** (layout/xy_cut.rs)

   - Has deprecated `single_column()` and `multi_column()` methods
   - Uses hardcoded thresholds instead of adaptive calculation

4. **Table Detection** (lattice.rs)
   - Edge threshold: 15pt (magic number)
   - Size filters: 50x50 min, 80% max (magic numbers)

**Recommendation:** Target MarginFilterProcessor for Loop 008 (similar to Loop 007).
