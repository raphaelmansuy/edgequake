# OODA Iteration 04 – Decide

**Date:** 2026-02-06
**Theme:** Superscript Detection via Position Analysis

## Decisions

1. Add `is_superscript(ref_font_size: f32) -> bool` method to `Span`.
2. Add `StyleType::Superscript` variant to the existing enum.
3. Create `get_style_type_with_ref(ref_font_size)` for context-aware detection.
4. Use `line.dominant_font_size()` as the reference font size.
5. Render superscript spans as `[text]` brackets in Markdown output.

## Detection Logic

```
is_superscript = (span.font_size / ref_font_size < 0.7)
                 && (span.text.len() < 5)
```

## Files to Modify

- `pymupdf_structs.rs` — add `is_superscript` method
- `pymupdf_renderer.rs` — add `Superscript` style type, update render pipeline

**Mission Re-read:** Confirmed.
