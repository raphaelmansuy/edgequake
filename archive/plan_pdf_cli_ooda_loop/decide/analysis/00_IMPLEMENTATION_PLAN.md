# DECIDE Phase: Implementation Strategy & Priorities

**Decision Date:** 2026-01-04  
**Based on:** OBSERVE + ORIENT findings  
**Goal:** Maximum impact with systematic, validated fixes  
**Approach:** Incremental, test-driven, validate each fix

---

## Fix Priority (Pareto Principle: 80% impact from 20% effort)

### P0: IMMEDIATE (Critical + Low Complexity)

#### Fix 1: Re-enable TableDetectionProcessor ⚡️

**Impact:** Fixes 100% of table structure loss  
**Effort:** 1-2 hours  
**Risk:** Low (can be disabled again if issues found)

**Action Plan:**

1. Uncomment TableDetectionProcessor in extractor.rs
2. Test on synthetic 03_tables.pdf
3. If issues found: debug specific cases
4. Add regression test
5. Validate table markdown output

**Success Criteria:**

- Tables rendered as `| header | header |` markdown
- At least 80% of tables correctly reconstructed
- No regressions on non-table content

---

#### Fix 2: Adjust Heading Detection Thresholds ⚡️

**Impact:** Fixes 60% of heading hierarchy loss  
**Effort:** 30 minutes  
**Risk:** Very Low

**Action Plan:**

1. Locate font size thresholds in StyleDetectionProcessor
2. Add support for H4 (size >= 11pt), H5 (size >= 10pt), H6 (size >= 9pt)
3. Test on 04_heading_hierarchy.pdf
4. Validate all 6 heading levels preserved

**Success Criteria:**

- All H1-H6 rendered as markdown headings (#, ##, etc.)
- No headings converted to bold text
- Hierarchy maintained

---

### P1: HIGH PRIORITY (High Impact + Medium Complexity)

#### Fix 3: List Structure and Indentation 📋

**Impact:** Fixes 90% of list formatting loss  
**Effort:** 2-4 hours  
**Risk:** Medium (complex logic)

**Investigation Steps:**

1. Enable debug logging for ListDetectionProcessor
2. Convert 02_lists_and_formatting.pdf with logging
3. Check Block metadata for level/indent values
4. Trace markdown renderer indentation logic
5. Identify gap in metadata → markdown translation

**Fix Strategies:**

- **If metadata correct, renderer wrong:** Fix render_list_item() indentation
- **If metadata wrong:** Fix list level calculation
- **If both wrong:** Fix detection + rendering

**Success Criteria:**

- Nested lists render with proper indentation (2 spaces per level)
- Bullet symbols consistent (`-` for unordered, `1.` for ordered)
- All nesting levels preserved

---

#### Fix 4: Font Style Detection (Bold/Italic) 💪

**Impact:** Fixes 50% of formatting loss  
**Effort:** 2-3 hours  
**Risk:** Medium

**Action Plan:**

1. Verify SOTA backend extracts font-weight/font-style
2. Check if StyleDetectionProcessor uses font properties
3. Ensure Span metadata includes style
4. Fix render_spans_styled() to apply \*_ and _ markers
5. Add inline code detection (monospace fonts)

**Implementation:**

```rust
// Pseudo-code
if font.weight > 600 || font.name.contains("Bold") {
    span.bold = true; // → render as **text**
}
if font.style == "italic" || font.name.contains("Italic") {
    span.italic = true; // → render as *text*
}
if font.family.contains("Mono") || font.family.contains("Courier") {
    span.code = true; // → render as `text`
}
```

**Success Criteria:**

- Bold text rendered as `**bold**`
- Italic text rendered as `*italic*`
- Inline code rendered as `` `code` ``
- Combined styles work (**_bold-italic_**)

---

### P2: MEDIUM PRIORITY (High Impact + High Complexity)

#### Fix 5: Unicode Character Encoding 🌍

**Impact:** Fixes 70% of special character corruption  
**Effort:** 4-6 hours  
**Risk:** High (encoding is complex)

**Investigation Steps:**

1. Review encodings.rs CMap decoding logic
2. Check if ToUnicode CMap is being read
3. Test with PDF containing Greek/math symbols
4. Add debug logging for character mapping

**Fix Strategies:**

- Ensure ToUnicode CMap takes priority
- Add fallback to standard encodings (WinAnsiEncoding, MacRomanEncoding)
- Handle multi-byte UTF-8 sequences
- Map common symbol glyph names (alpha → α, beta → β)

**Success Criteria:**

- Greek letters (α β γ) preserved
- Mathematical symbols (∀ ∃ ∈ ≈ ≠) preserved
- Currency symbols (₹ ₽ ₪) preserved
- Arrows (← → ↔ ⇒) preserved
- Emojis (😀 🎉) preserved or gracefully degraded

---

### P3: LOW PRIORITY (Polish)

#### Fix 6: Hyphenation Removal 🔧

**Impact:** Fixes 20% of text readability issues  
**Effort:** 1 hour  
**Risk:** Low

**Action Plan:**

1. Verify HyphenContinuationProcessor is running
2. Test pattern matching for "word-\nword" → "wordword"
3. Handle edge cases (legitimate hyphens: "well-known")
4. Add tests

**Success Criteria:**

- Hyphenated words at line breaks merged
- Legitimate hyphens preserved
- No text loss

---

#### Fix 7: Whitespace Normalization 📏

**Impact:** Fixes 10% of formatting issues  
**Effort:** 30 minutes  
**Risk:** Very Low

**Action Plan:**

1. Add option to preserve multiple spaces
2. Adjust clean_text() to be configurable
3. Test with special whitespace document

**Success Criteria:**

- Option to preserve/normalize whitespace
- Tabs handled consistently
- No loss of semantic whitespace

---

## Implementation Sequence

### Sprint 1: Critical Fixes (Day 1)

**Duration:** 2-3 hours  
**Goal:** Fix 90% of structural issues

```
1. Fix 1: Re-enable TableDetectionProcessor [1-2h]
   ├─ Uncomment line in extractor.rs
   ├─ Test on 03_tables.pdf
   ├─ Debug if needed
   └─ Validate output

2. Fix 2: Heading Thresholds [30min]
   ├─ Adjust font size ranges
   ├─ Test on 04_heading_hierarchy.pdf
   └─ Validate H4-H6

CHECKPOINT: Run full test suite, validate improvements
```

### Sprint 2: List and Style (Day 2)

**Duration:** 4-7 hours  
**Goal:** Fix 80% of formatting issues

```
3. Fix 3: List Structure [2-4h]
   ├─ Debug list detection
   ├─ Fix metadata preservation
   ├─ Fix renderer indentation
   └─ Test nested lists

4. Fix 4: Font Styles [2-3h]
   ├─ Extract font properties
   ├─ Map to markdown
   ├─ Test bold/italic/code
   └─ Validate combinations

CHECKPOINT: Run full test suite, validate improvements
```

### Sprint 3: Unicode and Polish (Day 3)

**Duration:** 5-7 hours  
**Goal:** Fix edge cases and encoding

```
5. Fix 5: Unicode Encoding [4-6h]
   ├─ Review CMap logic
   ├─ Add fallback encodings
   ├─ Test special characters
   └─ Validate full Unicode range

6. Fix 6: Hyphenation [1h]
7. Fix 7: Whitespace [30min]

CHECKPOINT: Run full test suite, validate 95%+ success rate
```

---

## Testing Strategy

### Unit Tests (Per Fix)

```rust
#[test]
fn test_table_detection_simple_2x3() {
    // Original markdown → PDF → IR → markdown
    // Assert table structure preserved
}

#[test]
fn test_heading_levels_h4_to_h6() {
    // Test all 6 heading levels
}

#[test]
fn test_nested_lists_three_levels() {
    // Test 3-level nested list
}
```

### Integration Tests (After Each Sprint)

```bash
# Re-run full test suite
for pdf in plan_pdf_cli_ooda_loop/observe/output/*.pdf; do
    ./convert_and_compare.sh "$pdf"
done

# Calculate improvement metrics
python3 calculate_fidelity_score.py
```

### Success Metrics

| Metric               | Baseline | Target Sprint 1 | Target Sprint 2 | Target Sprint 3 |
| -------------------- | -------- | --------------- | --------------- | --------------- |
| Table Structure      | 0%       | 80%             | 80%             | 85%             |
| List Structure       | 10%      | 10%             | 80%             | 85%             |
| Heading Hierarchy    | 40%      | 90%             | 90%             | 95%             |
| Font Styles          | 0%       | 0%              | 70%             | 75%             |
| Unicode Chars        | 30%      | 30%             | 30%             | 85%             |
| **Overall Fidelity** | **30%**  | **60%**         | **75%**         | **90%**         |

---

## Risk Mitigation

### Risk 1: TableDetectionProcessor still causes issues

**Mitigation:**

- Test incrementally on synthetic data first
- Add config flag to enable/disable
- Have fallback to TextTableReconstruction

### Risk 2: List detection breaks existing documents

**Mitigation:**

- Run regression tests on existing PDF suite
- Compare before/after metrics
- Have rollback plan

### Risk 3: Unicode fix introduces new encoding issues

**Mitigation:**

- Test on diverse character sets (Latin, Greek, CJK, Arabic)
- Have comprehensive test coverage
- Use well-tested encoding libraries

---

## Rollback Plan

If any fix causes regressions:

1. Revert the specific commit
2. Add regression test case
3. Analyze failure mode
4. Re-implement with better approach
5. Validate again

---

## Documentation Plan

After each sprint:

1. Update ACT phase log with:
   - What was fixed
   - How it was fixed
   - Test results
   - Remaining issues
2. Update git commits with:
   - Clear commit messages
   - Before/after examples
   - Test coverage

---

## Next Steps: ACT Phase

1. Start Sprint 1: Fix TableDetectionProcessor
2. Document every change in plan_pdf_cli_ooda_loop/act/
3. Run tests after each fix
4. Validate improvements with metrics
5. Loop back to OBSERVE if issues found

---

**Status:** DECIDE phase complete → Moving to ACT phase
