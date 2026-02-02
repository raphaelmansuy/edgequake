# OODA Iteration 03 - Decide

## Decision: Create Comprehensive Test Suite

### Chosen Approach

Create `quality_extraction.rs` with 8 tests covering all quality dimensions.

### Test Design

1. **Reading Order Tests** (test_qwen_reading_order)
   - Verify "Pushing" appears before "Beyond"
   - Validates flipped coordinate fix

2. **Content Tests** (test\_\*\_content)
   - Check minimum byte output
   - Verify key phrases present

3. **Structure Tests** (test*\*\_structure, test*\*\_headings)
   - Verify page count
   - Count heading markers

4. **Special Character Tests** (test_agentic_platform_code_blocks)
   - Check box-drawing character preservation

5. **Summary Test** (test_all_pdfs_extraction_summary)
   - Run all PDFs with pass/fail reporting
   - Quick validation of overall quality

### Success Criteria

All 8 tests pass with no failures.
