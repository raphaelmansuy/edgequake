# OODA Iterations 45-60: Improvement Roadmap

## Overview

These iterations focus on closing the quality gap from 0.786 → 0.95

---

## Quality Gap Analysis

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Quality Improvement Targets                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Metric        Current   Target    Gap      Focus Area                      │
│  ─────────────────────────────────────────────────────────                  │
│  ROUGE-L       0.832     0.95      0.118    Order preservation              │
│  Word F1       0.941     0.98      0.039    Content accuracy                │
│  Structure     0.417     0.90      0.483    *** BIGGEST GAP ***             │
│  Format        0.659     0.95      0.291    Bold/italic/lists               │
│                                                                             │
│  OVERALL       0.786     0.95      0.164                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Iteration Plan

### Phase 1: Code Quality (45-50)

| OODA | Focus                                       | Status  |
| ---- | ------------------------------------------- | ------- |
| 45   | Split pymupdf_grouper.rs (SRP)              | Planned |
| 46   | Extract column detection to separate module | Planned |
| 47   | Extract block classifier to separate module | Planned |
| 48   | Add WHY comments to grouping logic          | Planned |
| 49   | Add ASCII diagrams to layout algorithms     | Planned |
| 50   | Improve test coverage for grouper           | Planned |

### Phase 2: Structure Improvement (51-55)

| OODA | Focus                                           | Target Improvement |
| ---- | ----------------------------------------------- | ------------------ |
| 51   | Header detection: use font size + bold together | +0.05 Structure    |
| 52   | Section number detection (1.1, 2.3.1)           | +0.03 Structure    |
| 53   | List item indentation analysis                  | +0.05 Structure    |
| 54   | Code block detection (monospace + indentation)  | +0.02 Structure    |
| 55   | Table structure recognition                     | +0.03 Structure    |

### Phase 3: Format Improvement (56-60)

| OODA | Focus                                        | Target Improvement |
| ---- | -------------------------------------------- | ------------------ |
| 56   | Bold span detection (consecutive bold chars) | +0.05 Format       |
| 57   | Italic span detection                        | +0.03 Format       |
| 58   | Bullet point normalization (•, -, \*)        | +0.02 Format       |
| 59   | Numbered list continuation                   | +0.02 Format       |
| 60   | Citation/reference formatting                | +0.02 Format       |

---

## Success Criteria

Each OODA iteration must:

1. Have observe/orient/decide/act documentation
2. Include before/after quality metrics
3. Add unit tests for new functionality
4. Pass all existing tests
5. Not regress quality score

---

## Expected Outcome

After OODA-60:

- Structure: 0.417 → ~0.65 (+0.23)
- Format: 0.659 → ~0.80 (+0.14)
- Overall: 0.786 → ~0.85 (+0.06)
