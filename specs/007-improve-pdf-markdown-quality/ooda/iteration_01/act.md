# OODA Iteration 01 - Act

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Mission File**: `specs/007-improve-pdf-markdown-quality.md`
**Date**: 2026-02-05

---

## 1. Changes Implemented

### 1.1 Header/Footer Margin Filtering

**File**: `edgequake/crates/edgequake-pdf/src/layout/pymupdf_grouper.rs`

**Added Parameters to GroupingParams struct (lines 81-102)**:

```rust
/// OODA-IT01: Header margin height to exclude (in points).
/// Text within header_margin from page top (y < header_margin) is filtered out.
/// WHY: pymupdf4llm uses header_margin=50 to filter page numbers, chapter titles.
pub header_margin: f32,

/// OODA-IT01: Footer margin height to exclude (in points).
/// Text within footer_margin from page bottom is filtered out.
/// WHY: pymupdf4llm uses footer_margin=50 to filter footnotes, page numbers.
pub footer_margin: f32,

/// Page height for footer margin calculation.
pub page_height: f32,
```

**Updated Default Values (lines 113-127)**:

```rust
// OODA-IT01: Header margin = 50pt matches pymupdf4llm default
header_margin: 50.0,
// OODA-IT01: Footer margin = 50pt matches pymupdf4llm default
footer_margin: 50.0,
// OODA-IT01: Default page height (US Letter = 792pt)
page_height: 792.0,
```

**Added Filtering Logic in chars_to_spans (lines 241-257)**:

```rust
// OODA-IT01: Filter header margin text (page numbers, chapter titles at top)
if ch.y0 < self.params.header_margin {
    continue;
}

// OODA-IT01: Filter footer margin text (footnotes, page numbers at bottom)
if self.params.footer_margin > 0.0 && self.params.page_height > 0.0 {
    if ch.y1 > self.params.page_height - self.params.footer_margin {
        continue;
    }
}
```

### 1.2 New Unit Tests

**Added 3 new tests (lines 1282-1355)**:

| Test | Purpose |
|------|---------|
| `test_header_margin_filtering` | Verifies chars at y < header_margin are filtered |
| `test_footer_margin_filtering` | Verifies chars at y > (page_height - footer_margin) are filtered |
| `test_header_and_footer_margin_filtering` | Verifies both margins work together |

---

## 2. Test Results

**Before Changes**: 494 tests passing
**After Changes**: 497 tests passing (+3 new tests)

```
test result: ok. 497 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```

---

## 3. Code Quality

- **SRP**: ✅ Margin filtering is a single responsibility
- **DRY**: ✅ No code duplication
- **Dead Code**: ✅ No dead code introduced
- **Fast Tests**: ✅ New tests run in <1ms

---

## 4. Commit

```bash
git add -A && git commit -m "OODA-IT01: Add header/footer margin filtering

- Add header_margin, footer_margin, page_height to GroupingParams
- Filter chars in header region (y < header_margin)
- Filter chars in footer region (y > page_height - footer_margin)
- Add 3 unit tests for margin filtering
- Matches pymupdf4llm's column_boxes() margin parameters

WHY: Page chrome (headers, footers, page numbers) introduces noise
into the markdown output. Filtering improves extraction quality."
```

---

## 5. Files Changed

| File | Lines Changed | Type |
|------|---------------|------|
| `src/layout/pymupdf_grouper.rs` | +80 | Feature + Tests |

---

## 6. Next Iteration Focus

The margin filtering is now in place but requires the page height to be passed correctly through the pipeline. Iteration 02 will:

1. Verify margin filtering is applied in the extraction pipeline
2. Test with real documents (AI_Services__Elitizon.pdf)
3. Consider adjusting defaults if needed

---

## 7. Metrics

| Metric | Before | After | Δ |
|--------|--------|-------|---|
| Test count | 494 | 497 | +3 |
| Test time | 0.08s | 0.08s | 0 |
| Clippy warnings | TBD | TBD | - |
