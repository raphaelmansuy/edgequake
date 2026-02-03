# OODA-28 Decide: Fast Quality Tests for New PDFs

## Re-read Mission Status ✅

Re-read `specs/004-perfect-pdf-markdown-conversion.md` at iteration start.

## Decision: Add Fast Quality Tests with New PDFs

### Rationale (First Principles)

**Problem:** We have new PDFs in `zz_test_docs/` but no fast quality tests for them.

**Goal:** Measure extraction quality quickly (<5s total) on diverse document types.

**Strategy:**

1. Add tests for documents we extract WELL (validate our strengths)
2. Add tests for documents we extract POORLY (track improvement)
3. Keep each test <500ms to maintain fast feedback loop

### Specific Changes

#### 1. Add arXiv Paper Test (Our Strength)

**File:** `tests/fast_quality.rs`

**Test:** `test_arxiv_two_column_reading`

- PDF: Use existing `real_dataset/` paper or copy smaller one
- Check: Title extracted, abstract detected, reading order correct
- Time budget: 500ms

**Why:** Validates our column detection advantage over markitdown.

#### 2. Add Business Document Test

**File:** `tests/fast_quality.rs`

**Test:** `test_business_document_extraction`

- PDF: `Scottish SMEs Delegation*.pdf` (283KB) - clean single column
- Check: Key terms present, basic structure
- Time budget: 300ms

**Why:** Validates simple document extraction quality.

#### 3. Add Encoding Challenge Test (Expected Partial Failure)

**File:** `tests/fast_quality.rs`

**Test:** `test_encoding_challenge_apple_sandbox`

- PDF: `Apple-Sandbox-Guide-v1.0.pdf` (354KB)
- Check: Non-zero output, measure what % is readable
- Time budget: 500ms
- Note: Document known encoding issues for future fix

**Why:** Creates baseline for tracking encoding improvements.

#### 4. Create Markitdown Gold Standards

Save markitdown output as gold files for:

- `Scottish SMEs*.pdf` → `test-data/scottish_smes.gold.md`

**Why:** Enables automated comparison in tests.

### Implementation Plan

```rust
// New test structure in fast_quality.rs

#[tokio::test]
async fn test_arxiv_two_column_reading() {
    // Uses real_dataset paper
    // Validates: title present, reasonable word count
    // Threshold: extraction non-empty, >1000 chars
}

#[tokio::test]
async fn test_business_document_extraction() {
    // Uses Scottish SMEs PDF
    // Validates: key terms present
    // Threshold: TPS > 80%, time < 500ms
}

#[tokio::test]
async fn test_encoding_quality_tracking() {
    // Uses Apple-Sandbox-Guide
    // Tracks: % of readable ASCII characters
    // Creates baseline for future improvement
}
```

### Test Time Budget

| Existing Tests                     | Time   |
| ---------------------------------- | ------ |
| test_text_preservation_fast        | 1609ms |
| test_structure_detection_fast      | 1616ms |
| test_simple_table_fast             | 148ms  |
| test_two_column_reading_order_fast | 234ms  |
| test_fast_quality_summary          | 0ms    |

Note: test_text_preservation_fast and test_structure_detection_fast are the same PDF
extracted twice. Total unique extraction time: ~1.6s + 0.15s + 0.23s = ~2s

| New Tests                         | Estimated Time |
| --------------------------------- | -------------- |
| test_arxiv_two_column_reading     | 400ms          |
| test_business_document_extraction | 300ms          |
| test_encoding_quality_tracking    | 400ms          |

**Total estimated:** ~3.1s (under 5s target ✅)

### Non-Changes (Deferred)

1. **Font encoding fix** - Too complex for this iteration
2. **Comprehensive test updates** - Focus on fast tests
3. **AGL lookup implementation** - Needs dedicated research

## Success Criteria

1. ✅ All new tests pass (with appropriate thresholds)
2. ✅ Total fast_quality.rs time < 5 seconds
3. ✅ New PDFs provide diverse coverage
4. ✅ Encoding test creates baseline for future tracking
