# OODA Iteration 01: Decide

**Date**: 2026-02-06
**Previous Phase**: orient.md completed
**Mission Re-read**: ✅ Confirmed

---

## Objective

Prioritize specific, actionable changes for Iteration 01 based on signal value, impact, and feasibility. Make decisions on what to implement **now** vs **later**.

---

## Iteration 01 Scope: Foundation & Quick Wins

**Philosophy**: Start with **low-effort, high-value** changes that:

1. Establish baseline metrics
2. Add missing infrastructure for future work
3. Deliver immediate user value
4. Don't break existing functionality

**Anti-Pattern**: Avoid "big bang" changes that touch many files.

---

## Decision Matrix

### Tier 0: Infrastructure (MUST DO)

**Purpose**: Enable future improvements without delivering direct user value yet.

#### Decision 1.1: Establish Baseline Metrics ✅

**What**: Run comprehensive test suite and document current quality metrics.

**Why**: Cannot measure improvement without knowing starting point.

**Effort**: 15 minutes
**Value**: Foundation for all future iterations
**Risk**: None (read-only operation)

**Action**:

```bash
cd edgequake/crates/edgequake-pdf
cargo test --all-features 2>&1 | tee baseline_iter01.txt
cargo test --test comprehensive_quality --features comprehensive-tests\
  2>&1 | tee quality_baseline_iter01.txt
```

**Success Criteria**:

- [ ] All tests pass (or document failures)
- [ ] Metrics captured: accuracy, table detection, etc.
- [ ] Baseline file committed to specs/

#### Decision 1.2: Add char_flags Field to Span ✅

**What**: Extend Span struct to include char-level font flags.

**Why**: Required for strikeout detection (char_flags & 0x01).

**Effort**: 30 minutes
**Value**: Enables future style improvements
**Risk**: Low (additive change, existing code unaffected)

**Files to Modify**:

- `src/backend/elements.rs`: Add `char_flags: u32` to Span struct
- `src/backend/pdfium.rs`: Extract char flags from PDFium
- `src/layout/pymupdf_structs.rs`: Propagate field if needed

**Success Criteria**:

- [ ] Span has char_flags field
- [ ] PDFium extraction populates field correctly
- [ ] Tests pass (no regressions)

---

### Tier 1: PUA Character Filtering (DO NOW)

**Rationale**: Low effort, medium value, no dependencies.

#### Decision 1.3: Implement PUA Character Detection ✅

**What**: Add function to detect and filter Private Use Area Unicode characters.

**Why**: PDFs use PUA for custom bullets/symbols → garbage in output if not filtered.

**Effort**: 1 hour
**Value**: Immediate quality improvement for documents with custom fonts
**Risk**: Very low (pure function, easy to test)

**Implementation**:

```rust
// New file: src/renderers/pua_filter.rs

/// Check if character is in Private Use Area (PUA) of Unicode.
///
/// PUA ranges:
/// - BMP: U+E000..U+F8FF
/// - Supplementary PUA-A: U+F0000..U+FFFFD
/// - Supplementary PUA-B: U+100000..U+10FFFD
///
/// REF: pymupdf4llm document_layout.py:83-94
pub fn is_pua_char(c: char) -> bool {
    let code_point = c as u32;
    matches!(code_point,
        0xE000..=0xF8FF |
        0xF0000..=0xFFFFD |
        0x100000..=0x10FFFD
    )
}

/// Remove PUA characters from text.
pub fn filter_pua(text: &str) -> String {
    text.chars()
        .filter(|&c| !is_pua_char(c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pua_detection() {
        // BMP PUA
        assert!(is_pua_char('\u{E000}'));
        assert!(is_pua_char('\u{F8FF}'));

        // Supplementary PUA-A
        assert!(is_pua_char('\u{F0000}'));
        assert!(is_pua_char('\u{FFFFD}'));

        // Normal characters
        assert!(!is_pua_char('A'));
        assert!(!is_pua_char('•'));  // U+2022 (not PUA)
        assert!(!is_pua_char('→'));  // U+2192 (not PUA)
    }

    #[test]
    fn test_pua_filtering() {
        let input = "Hello\u{E001}World";
        assert_eq!(filter_pua(input), "HelloWorld");

        let input = "Normal text";
        assert_eq!(filter_pua(input), "Normal text");
    }
}
```

**Integration Points**:

- `src/renderers/markdown.rs`: Apply filter when rendering spans
- `src/layout/pymupdf_renderer.rs`: Apply in render_span_styled

**Success Criteria**:

- [ ] Tests pass for PUA detection
- [ ] Documents with PUA bullets render cleanly
- [ ] No regression on documents without PUA

**Test Case**: Find PDF with custom bullets (check test-data/).

---

### Tier 2: Extended Style Preservation (DO NOW)

**Rationale**: Medium effort, high value, builds on Tier 1.

#### Decision 1.4: Add Superscript and Strikeout Rendering ✅

**What**: Extend markdown renderer to handle superscript (flags & 0x01) and strikeout (char_flags & 0x01).

**Why**: Footnotes and edited text currently lose meaning.

**Effort**: 2 hours
**Value**: High - improves readability and semantic accuracy
**Risk**: Medium (changes rendering logic, could affect existing output)

**Implementation**:

```rust
// Modify: src/layout/pymupdf_renderer.rs

fn render_span_styled(&self, span: &Span) -> String {
    // Decode flags (order matters for nesting!)
    let is_superscript = span.flags & 0x01 != 0;
    let is_italic = span.flags & 0x02 != 0;
    let is_mono = span.flags & 0x08 != 0 && span.font != "GlyphLessFont";
    let is_bold = span.flags & 0x10 != 0 || span.char_flags & 0x08 != 0;
    let is_strikeout = span.char_flags & 0x01 != 0;

    // Filter PUA characters first
    let text = filter_pua(&span.text);
    if text.is_empty() {
        return String::new();
    }

    // Handle superscript (common for footnotes)
    if is_superscript && text.len() < 5 {
        return format!("[{}]", text);  // [1], [*], etc.
    }

    // Build markdown prefix (innermost to outermost)
    let mut prefix = String::new();
    if is_mono { prefix.push('`'); }
    if is_bold { prefix.push_str("**"); }
    if is_italic { prefix.push('_'); }
    if is_strikeout { prefix.push_str("~~"); }

    // Suffix is reverse of prefix
    let suffix: String = prefix.chars().rev().collect();

    format!("{}{}{}", prefix, text, suffix)
}
```

**Files to Modify**:

- `src/layout/pymupdf_renderer.rs`: Update render_span_styled
- `src/backend/elements.rs`: Ensure Span has char_flags field (from Decision 1.2)

**Success Criteria**:

- [ ] Footnote markers render as [1], [*], etc.
- [ ] Strikeout text renders with ~~ markers
- [ ] Existing bold/italic tests still pass
- [ ] New test: footnote PDF renders correctly

**Test Case**: Create micro-test with footnote superscripts.

---

### Tier 3: Documentation & Planning (DO NOW)

**Rationale**: Low effort, high long-term value.

#### Decision 1.5: Document List Hierarchy Algorithm ✅

**What**: Write detailed specification for list hierarchy detection to implement in Iteration 02.

**Why**: Complex algorithm needs careful design before implementation.

**Effort**: 1 hour
**Value**: Guides future work, avoids rework
**Risk**: None (documentation only)

**Deliverable**: Create `specs/001-improve-markdown-2-pdf/list_hierarchy_design.md` with:

- Algorithm pseudocode
- Example inputs/outputs
- Edge cases to handle
- Test strategy

**Success Criteria**:

- [ ] Design doc completed
- [ ] Reviewed against pymupdf4llm:97-151
- [ ] Edge cases identified

**Defer Implementation**: Iteration 02 (higher complexity, needs Tier 0-2 foundation first).

---

## Decisions DEFERRED to Later Iterations

### Defer 1.6: Hyphenation Resolution → Iteration 03

**Why**: Needs extensive testing to avoid false positives on lists.

**Dependencies**: None, but lower priority than style fixes.

---

### Defer 1.7: Expanded Block Types → Iteration 04-05

**Why**: Requires confidence scoring infrastructure (complex).

**Dependencies**: Need baseline metrics first (Decision 1.1).

---

### Defer 1.8: Table Structure Completion → Iteration 11-15

**Why**: High complexity, requires vector graphics extraction.

**Dependencies**: Core layout must be solid first.

---

### Defer 1.9: OCR Integration → Iteration 30+

**Why**: Low priority for most users, high implementation cost.

**Dependencies**: All core features should work well first.

---

## Implementation Order for Iteration 01

**Total Effort Estimate**: 5-6 hours

### Phase A: Infrastructure (90 minutes)

1. ✅ Run baseline tests (15 min)
2. ✅ Add char_flags to Span (30 min)
3. ✅ Commit: "OODA-01: Add char_flags infrastructure for extended styling"

### Phase B: PUA Filtering (60 minutes)

4. ✅ Implement pua_filter.rs (30 min)
5. ✅ Add tests (15 min)
6. ✅ Integrate into renderers (15 min)
7. ✅ Commit: "OODA-01: Filter Private Use Area characters from output"

### Phase C: Style Extension (120 minutes)

8. ✅ Extend render_span_styled (45 min)
9. ✅ Add micro-tests for superscript/strikeout (30 min)
10. ✅ Integration testing (30 min)
11. ✅ Commit: "OODA-01: Add superscript and strikeout style preservation"

### Phase D: Documentation (60 minutes)

12. ✅ Write list hierarchy design doc (60 min)
13. ✅ Commit: "OODA-01: Design spec for list hierarchy detection"

### Phase E: Validation (30 minutes)

14. ✅ Run full test suite (10 min)
15. ✅ Document results vs baseline (10 min)
16. ✅ Write act.md with results (10 min)

---

## Rollback Plan

**If tests fail after changes**:

1. **Identify failing test**: `cargo test <test_name>`
2. **Check if new feature**: If testing new functionality, fix forward
3. **Check if regression**: If existing test broke, git revert:
   ```bash
   git log --oneline -10  # Find commit hash
   git revert <hash>      # Revert specific commit
   ```
4. **Document in act.md**: Note what failed, why, and resolution

---

## Success Criteria for Iteration 01

**Mandatory** (must all pass):

- [ ] All existing tests pass (no regressions)
- [ ] char_flags field added and populated correctly
- [ ] PUA filtering works on test cases
- [ ] Superscript renders as brackets
- [ ] Strikeout renders with ~~ markers
- [ ] Baseline metrics documented
- [ ] List hierarchy design completed
- [ ] All code committed with proper format: "OODA-01: <description>"

**Optional** (stretch goals):

- [ ] Find real PDF with PUA chars and validate
- [ ] Benchmark: no performance regression
- [ ] Update README with new capabilities

---

## Risk Mitigation

### Risk: Breaking existing rendering

**Mitigation**:

- Run `cargo test --features comprehensive-tests` after each commit
- If failure: immediately revert and debug
- Keep commits atomic (one feature per commit)

### Risk: PDFium doesn't provide char_flags

**Mitigation**:

- Check pdfium-render API docs first
- If unavailable: Set char_flags = 0 (no-op)
- Document limitation: "Strikeout requires PDFium char flags"

### Risk: Time overrun (>6 hours)

**Mitigation**:

- If Phase C takes >2 hours: Defer to Iteration 02
- If Phase D takes >1 hour: Write abbreviated design
- Priority: Infrastructure > Style > Docs

---

## Next Phase

In **act.md**, will document:

- Exact code changes made with file:line references
- Commit SHAs for each change
- Test results comparing baseline to post-changes
- Issues encountered and resolutions
- Evidence that success criteria met

---

**Decisions Finalized**: ✅

- [x] Scope defined: 5 decisions, 5-6 hours effort
- [x] Implementation order planned
- [x] Success criteria clear
- [x] Rollback plan documented
- [x] Risks identified and mitigated
