# OODA-18 Orient: Gold File Dependency Analysis

## Key Finding

The gold files were created from historical Y-sorted (interleaved) output, not from ideal reading order.

## Implications

### Attempted Fixes Failed Quality Checks

1. **Preserve original order for non-table blocks**: Quality dropped from 86.5% to 86.1%
2. **Skip table detection for multi-column pages**: Quality dropped from 86.5% to 83.1%

### Root Cause of Quality Drop

The "fix" produces CORRECT reading order (left column, then right column), but this no longer matches the gold files which have INTERLEAVED order.

Example from `01_2512.25075v1.gold.md`:

```
...disentangles space and time...      (LEFT COLUMN)
...independently alter the camera...   (LEFT COLUMN)
...re-rendering the                    (LEFT COLUMN)
scene for continuous and arbitrary...  (RIGHT COLUMN - INTERLEAVED!)
```

## Trade-offs

| Approach            | Quality Score | Actual Correctness |
| ------------------- | ------------- | ------------------ |
| Baseline (Y-sorted) | 86.5%         | Scrambled columns  |
| Reading order fix   | 86.1%         | Correct columns    |

## Strategic Options

1. **Update gold files** (High effort, high reward)
   - Manually fix gold files to use correct reading order
   - Future fixes would properly improve quality

2. **Add reading order metric** (Medium effort)
   - Track reading order correctness separately
   - Decouple from text similarity scoring

3. **Accept current state** (Low effort)
   - Y-sorted output is "good enough"
   - Focus on other improvements
