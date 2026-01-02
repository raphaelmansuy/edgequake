# DECIDE - Loop 012

## Timestamp

Fri Jan 2, 2026 19:35:00 HKT

## Directory Scope

**Target: crates/edgequake-pdf/src/processors/processor.rs**

## Root Cause (Confirmed)

### Issue: Title Not Detected as H1 Due to Length Threshold

**Location:** `processor.rs` line 2225

```rust
let is_short_for_heading =
    text.len() < 80 && !text.ends_with('.') && !has_inline_description;
```

**Problem:**

- Document titles are often 80-120 characters
- Example: "LLMS4OL 2025: The 2nd Large Language Models for Ontology Learning Challenge at the 24th ISWC" = 93 chars
- Current 80-char threshold excludes most academic paper titles
- This causes titles to be rendered as plain text instead of H1

**Evidence:**

```
Gold:     # LLMS4OL 2025: The 2nd Large Language Models...
Generated: LLMs4OL 2025: The 2nd Large Language Models...  (no # marker)
```

## Proposed Patch (Minimal, High ROI)

### Change 1: Differentiate Title vs Section Header Thresholds

**Rationale (First Principles):**

1. **Document titles** (H1) are typically longer than section headers
   - Titles include full context: topic, venue, year
   - Average academic title: 80-120 characters
2. **Section headers** (H2, H3) are shorter
   - "Introduction", "Methods", "1. Background" etc.
   - Average: 20-50 characters
3. **Single length threshold** doesn't account for this natural hierarchy

### Implementation

```rust
// Line 2225 - Replace single threshold with level-specific thresholds:

// OLD:
let is_short_for_heading =
    text.len() < 80 && !text.ends_with('.') && !has_inline_description;

// NEW:
// For H1 detection (titles), allow up to 120 chars
// For H2/H3 detection (sections), keep 80 chars
// This reflects natural heading length hierarchy
```

### Change 2: Add Title Position Heuristic

**Rationale:**

- Document titles typically appear in the first 10% of the first page
- Titles are the largest text on the page
- Combining position + size gives higher confidence

**Implementation:**

```rust
// Line 2310-2330 - Add position-aware title detection:

// Check if this is the first page and block is near the top
let is_first_page = page.number == 1;
let block_y = block.bbox.as_ref().map(|b| b.y1).unwrap_or(0.0);
let is_top_of_page = block_y < 200.0; // Top 25% of typical 800pt page

// For first page top blocks, use relaxed length threshold for H1
let max_heading_len = if is_first_page && is_top_of_page && size > body_size * 1.5 {
    120  // Allow longer text for document title
} else {
    80   // Standard threshold for section headers
};

let is_short_for_heading =
    text.len() < max_heading_len && !text.ends_with('.') && !has_inline_description;
```

## Predicted Impact

### Metrics Improvement

- **Before**: Composite 32.5/100 (Style 31.5%, Table 2.4%)
- **Target**: Composite 36-38/100 (Style 38-42%, Table 2.4%)

### Drift Reduction

- **heading:mismatch**: 82 → ~20 (-75%)
- **content:mismatch**: 2067 → ~1950 (-6% indirect effect)

**Reasoning:**

- Fixing H1 detection will correctly mark ~60 title blocks as headers
- This cascades to improved markdown structure recognition
- Style accuracy will improve due to better hierarchical structure

## Acceptance Checklist

### 1. Unit Test - New Test for Long Titles

```rust
#[test]
fn test_long_title_detection() {
    // Title: 93 characters (typical academic paper)
    let title = "LLMS4OL 2025: The 2nd Large Language Models for Ontology Learning Challenge at the 24th ISWC";

    // Should be detected as H1 despite length > 80
    // Requirements: first page, large font, top position
}
```

### 2. Regression Tests

- ✅ All existing 111 tests must pass
- ✅ Short section headers (< 80 chars) still detected as H2/H3

### 3. Validator Metrics

- ✅ Style Accuracy improves by at least 3 percentage points (31.5% → 34.5%+)
- ✅ heading:mismatch drifts decrease by at least 50% (82 → 41 or fewer)
- ✅ No regression in Table Accuracy (stay at 2.4% or improve)

### 4. Real Dataset Validation

Run validator on all 5 PDFs:

```bash
cargo run -p edgequake-pdf --example real_dataset_eval -- --write
python3 .github/skills/pdf-markdown-validator/scripts/validate.py \
  --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset \
  --gold-dir edgequake/crates/edgequake-pdf/test-data/real_dataset \
  --output-report sessions/improve_pdf/metrics_loop_012_after.json
```

### 5. Specific Document Checks

- ✅ 2900_Goyal_et_al: Title rendered as `# LLMS4OL 2025...`
- ✅ AlphaEvolve: Title rendered with proper H1 marker
- ✅ No false positives: Long paragraph text not converted to headers

## Alternative Approaches (Considered but Rejected)

### Option A: Machine Learning Classifier

**Pros:** Could learn complex patterns
**Cons:** Adds dependencies, training data requirement, overkill for this problem
**Verdict:** ❌ Violates first-principles simplicity

### Option B: Regex-Based Title Pattern Matching

**Pros:** Could catch specific title patterns
**Cons:** Brittle, requires maintaining keyword lists, domain-specific
**Verdict:** ❌ Violates first-principles (uses heuristics)

### Option C: Selected - Adaptive Length Threshold

**Pros:** Simple, geometric (position + size), no heuristics
**Cons:** May need tuning for edge cases
**Verdict:** ✅ **Best balance of simplicity and effectiveness**

## Risk Assessment

### Low Risk

- Change is surgical (3 lines of code)
- Only affects heading detection logic
- Doesn't modify extraction or rendering layers
- Backward compatible (existing tests should pass)

### Mitigation

- Add comprehensive unit tests for edge cases
- Monitor for false positives (paragraphs marked as headers)
- Can easily revert if issues arise

## Implementation Time Estimate

- Code changes: 10 minutes
- Unit tests: 15 minutes
- Full test run: 5 minutes
- Validation: 10 minutes
- **Total: ~40 minutes**

## Next Step

Proceed to ACT phase to implement the patch.
