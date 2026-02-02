# OODA-11 OBSERVE: Structural Fidelity Gap Analysis

## Current Metrics (After OODA-10)

- Text Preservation: 83.5%
- Structural Fidelity: 69.0%
- Overall: 76.2%

## MAJOR FINDING: Structural Element Count Comparison (v2 PDF)

| Element        | Gold   | Extracted | Missing | % Gap           |
| -------------- | ------ | --------- | ------- | --------------- |
| Headers        | 27     | 19        | 8       | 30% missing     |
| Tables         | 45     | 17        | 28      | 62% missing     |
| **List Items** | **85** | **22**    | **63**  | **74% missing** |

## ROOT CAUSE: Reference List Extraction Failure

### Gold Standard (correct)

```markdown
- [1] L. Sentis and O. Khatib, "A whole-body control framework..."
- [2] X. Cheng, J. Li, S. Yang, "Open-television..."
- [3] X. Gu, Y.-J. Wang, and J. Chen, "Humanoid-gym..."
```

### Current Extraction (broken)

```markdown
## REFERENCES

humanoids operating in human environments," in ICRA., 2006. Teleoperation...
[26] Z. Fu, Q. Zhao, Q. Wu, G. Wetzstein...
[3] X. Gu, Y.-J. Wang, and J. Chen, "Humanoid-gym...
[27] Y. Ze, Z. Chen, J. P. Araujo...
```

### Problems Identified

1. **References NOT detected as list items** - missing `* ` prefix
2. **Two-column cross-merge** - [26] and [3] are in wrong order (from different columns)
3. **Line breaks lost** - entries run into each other
4. **44 reference items completely missing** from structural count

## WHY This Matters

The structural scoring algorithm counts:

- Headers starting with `#`
- Lists starting with `-`, `* `, or digits
- Tables containing `|`

References in gold: 44 × `* [n]` patterns → counted as list items
References in extraction: 0 × `* [n]` patterns → NOT counted

This alone accounts for ~52% of the list item gap (44 of 85 missing).

## Next: ORIENT Phase

1. Trace how references get processed through the pipeline
2. Identify where two-column merging goes wrong
3. Find where list markers should be detected
