# OODA-06 Act: Line Break Preservation

## Change Implemented

**File**: `layout/pymupdf_renderer.rs`
**Function**: `render_lines_inline`

Changed line joining from space to newline:

```rust
// Before: .join(" ")
// After:  .join("\n")
```

## Evaluation Results

| Metric      | OODA-05 | OODA-06 | Delta  |
| ----------- | ------- | ------- | ------ |
| **Quality** | 0.702   | 0.724   | +3.1%  |
| ROUGE-L     | 0.701   | 0.701   | +0.0%  |
| Word F1     | 0.897   | 0.897   | +0.0%  |
| Structure   | 0.453   | 0.602   | +32.9% |
| Format      | 0.470   | 0.470   | +0.0%  |

## Per-File Improvements

| File                  | Before | After | Delta  |
| --------------------- | ------ | ----- | ------ |
| agent_2510.09244v1    | 0.870  | 0.951 | +9.3%  |
| one_tool_2512.20957v2 | 0.493  | 0.668 | +35.5% |
| AlphaEvolve           | 0.478  | 0.602 | +25.9% |

## Analysis

✅ **Structure score significantly improved** (+33%) - Lines now match gold format
✅ **agent_2510 Structure now at 0.951** - Near-perfect structural match
⚠️ **ROUGE-L unchanged** - Line breaks don't affect LCS word-level matching

## Next Focus

- **ROUGE-L still at 0.701** (target: 0.90) - Reading order needs work
- **Format at 0.470** (target: 0.70) - Italic/list detection needs improvement
- **v2_2512 still lowest** at 0.582 - Deep dive needed
