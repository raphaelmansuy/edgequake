# OODA Iteration 01 - Decide

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Mission File**: `specs/007-improve-pdf-markdown-quality.md`
**Date**: 2026-02-05

---

## 1. Decision: Add Header/Footer Margin Filtering

### 1.1 Scope

Add configurable header and footer margin parameters to filter out noise from:
- Page headers (page numbers, chapter titles)
- Page footers (footnotes, copyright)
- Running headers that span columns

### 1.2 Implementation Plan

**Location**: `edgequake-pdf/src/layout/pymupdf_grouper.rs`

**Changes Required**:

1. **Add margin parameters to GroupingParams**
   ```rust
   pub struct GroupingParams {
       pub header_margin: f32,  // NEW: pixels from top to ignore
       pub footer_margin: f32,  // NEW: pixels from bottom to ignore
       // ... existing fields
   }
   ```

2. **Filter elements before grouping**
   - Filter out elements where `y < header_margin`
   - Filter out elements where `y > (page_height - footer_margin)`

3. **Add configuration options to Config**
   - Default header_margin: 50.0
   - Default footer_margin: 50.0
   - Make configurable via CLI

### 1.3 Files to Modify

| File | Change |
|------|--------|
| `src/layout/pymupdf_grouper.rs` | Add margin filtering |
| `src/config.rs` | Add margin config options |
| `src/extractor.rs` | Pass margins to grouper |

### 1.4 Tests to Add

1. `test_margin_filtering_basic` - Verify elements filtered correctly
2. `test_margin_filtering_preserves_content` - Ensure main content not lost
3. `test_margin_defaults` - Verify sensible defaults

### 1.5 Acceptance Criteria

- [ ] Header margin filters top N pixels
- [ ] Footer margin filters bottom N pixels
- [ ] Defaults are sensible (50px each)
- [ ] All existing tests pass
- [ ] New tests cover margin logic
- [ ] AI_Services__Elitizon.pdf output improved

---

## 2. Priority Order

1. **P0**: Implement margin filtering in pymupdf_grouper.rs
2. **P1**: Add configuration support
3. **P2**: Add unit tests
4. **P3**: Validate with test document

---

## 3. Non-Goals for This Iteration

- Reading order refactoring (Phase 2)
- Rectangle joining (Phase 2)
- Pipeline reordering (Phase 3)
- Table detection improvements (Phase 4)

---

## 4. Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Test pass rate | 494/494 | 494/494 |
| AI_Services output quality | 60/100 | 70/100 |
| Code coverage | - | Maintain or improve |

---

## 5. Rollback Plan

If margin filtering causes issues:
1. Revert changes via git
2. Document failure mode
3. Adjust approach in next iteration
