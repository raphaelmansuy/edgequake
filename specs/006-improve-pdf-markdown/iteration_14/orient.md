# OODA-14: Orient - Add Heading Level Tests

## Analysis

### Current Test Coverage

| Ratio | Expected Type | Tested? |
|-------|---------------|---------|
| >= 2.0 | Header(1) | ✅ 24pt/12pt |
| 1.7-2.0 | Header(2) | ❌ Missing |
| 1.5-1.7 | Header(1) | ❌ Missing |
| < 1.5 | Paragraph | ❌ Missing |

### Test Cases to Add

1. **H2 test**: 18pt on 10pt body = 1.8x → Header(2)
2. **Edge H1 test**: 16pt on 10pt body = 1.6x → Header(1)
3. **Paragraph test**: 10pt on 10pt body = 1.0x → Paragraph

### Test Structure

Add new test function `test_heading_level_classification` that specifically tests the ratio thresholds.

## Prioritization

1. H2 test - critical, no coverage
2. Paragraph test - important, validates baseline
3. Edge cases - nice to have
