# OODA-27 Decide: Prioritized Action Plan

## Re-read Mission Status ✅

Re-read `specs/004-perfect-pdf-markdown-conversion.md` at iteration start.

## Decision Matrix

Based on the orient phase analysis, here are the prioritized actions:

| Priority | Action                                | Signal Value | Effort | Impact |
| -------- | ------------------------------------- | ------------ | ------ | ------ |
| **1**    | Create fast quality metric test       | HIGH         | LOW    | HIGH   |
| **2**    | Test current extraction vs markitdown | HIGH         | LOW    | MEDIUM |
| **3**    | Investigate column boundary filtering | HIGH         | MEDIUM | HIGH   |
| **4**    | Fix text truncation root cause        | HIGH         | MEDIUM | HIGH   |
| **5**    | Optimize line merging O(l²) → O(l)    | MEDIUM       | HIGH   | LOW    |

## Priority 1: Fast Quality Metric Test

**WHY:** The user explicitly requested this. Current tests take 118s and crash VS Code. We need instant feedback.

**WHAT:** Create a new test file `tests/fast_quality.rs` with:

1. **Test: `test_text_preservation_fast`**
   - PDF: `AI_Services__Elitizon.pdf` (110KB, 5 pages, clean structure)
   - Gold: Compare against markitdown output
   - Metric: Word overlap ratio (TPS - Text Preservation Score)
   - Target: >95% word match
   - Time: <500ms

2. **Test: `test_column_reading_fast`**
   - PDF: Create small excerpt from stackplanner (first page only)
   - Metric: Reading order correctness (no interleaved columns)
   - Time: <200ms

3. **Test: `test_table_detection_fast`**
   - PDF: Use existing small lattice test PDF
   - Metric: Table cell accuracy
   - Time: <100ms

**HOW:**

```rust
#[test]
fn test_text_preservation_fast() {
    // Extract in <500ms
    // Compare word-by-word with markitdown gold
    // Assert TPS >= 0.95
}
```

## Priority 2: Test Current Extraction vs Markitdown

**WHY:** Need baseline comparison to quantify improvement opportunities.

**WHAT:**

1. Use markitdown MCP to extract `AI_Services__Elitizon.pdf`
2. Use our extractor to extract same PDF
3. Compare outputs programmatically

**Expected Outcome:**

- Markitdown: Clean structure (we observed this)
- Ours: May have issues with simple single-column documents

## Priority 3: Investigate Column Boundary Filtering

**WHY:** Orient phase identified 15pt margin zone as potential text loss source.

**WHAT:** Read `text_grouping.rs` and trace what happens to elements near column boundary.

**WHERE:** `edgequake/crates/edgequake-pdf/src/backend/text_grouping.rs`

## Decision: What NOT to Do This Iteration

1. **Don't refactor line merging** - Low priority, high effort
2. **Don't add new PDF test categories** - Focus on quality, not quantity
3. **Don't change column detection algorithm** - Verify issue first

## Implementation Order

```
┌─────────────────────────────────────────────────────────────┐
│ Step 1: Create fast_quality.rs test file                    │
│   - Add 3 fast tests                                        │
│   - Verify they complete in <5s total                       │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 2: Run fast tests, capture baseline metrics            │
│   - Current TPS%                                            │
│   - Current SFS%                                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 3: Analyze column boundary handling                    │
│   - Read text_grouping.rs:group_two_column_layout()         │
│   - Identify where text is lost                             │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ Step 4: If issue found, fix and re-run fast tests           │
│   - Verify improvement in TPS                               │
└─────────────────────────────────────────────────────────────┘
```

## Success Criteria for This Iteration

1. ✅ `fast_quality.rs` exists and runs in <5 seconds
2. ✅ Baseline quality metrics captured
3. ✅ Column boundary issue investigated (fix if time permits)
4. ✅ All existing tests still pass

## Commit Strategy

- Commit 1: `OODA-27: Add fast quality metric tests`
- Commit 2: `OODA-27: [TBD based on findings]`
