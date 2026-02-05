# OODA-51: Header Detection Refinement

## Date: 2026-02-05 (Planned)

## Observe

Structure score is 0.417 (target: 0.90).

### Current Header Detection

- Uses font size ratio > 1.50x body
- Limited to 2 lines, 150 chars max
- Only H1 and H2 levels detected

### Issues

- Many headers missed (false negatives)
- Some non-headers marked as headers (false positives)

## Orient

Need to calibrate detection thresholds against gold standards.

## Decide

Analyze gold files to determine optimal thresholds.

## Act

**Status:** PLANNED

Changes to make:

1. Analyze gold markdown for header patterns
2. Adjust `header_ratio` in `BlockClassifier`
3. Add H3-H6 support based on font size ranges
4. Validate against quality tests

**Expected Impact:** Structure 0.417 → 0.50
