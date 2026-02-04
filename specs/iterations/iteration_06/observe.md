# OODA-06 Observe: Line Rendering Analysis

## Current State

- Quality: 0.702 (target ≥0.95)
- Structure: 0.453 (Lines component: 0.106)
- Format: 0.470

## Key Finding: Line Count Mismatch

### File: v2_2512.25072v1

- **Gold lines**: 1181
- **Our lines**: 620
- **Ratio**: 0.525 (almost half)

### Root Cause: Line Joining Algorithm

**Current behavior** (`pymupdf_renderer.rs:156`):

```rust
fn render_lines_inline(&self, lines: &[Line]) -> String {
    lines
        .iter()
        .map(|l| self.render_line_styled(l))
        .collect::<Vec<_>>()
        .join(" ")  // <-- Problem: joins with space
}
```

**pymupdf4llm behavior** (document_layout.py):

```python
output += line_text.rstrip() + "\n"  # Each line ends with newline
```

### Visual Comparison

**Gold output** (lines preserved):

```
Policy consistently outperforms both diffusion policy and
behavior cloning with action chunking. We further perform
ablation studies demonstrating that learned score-based se-
lection is significantly more effective than baselines.
```

**Our output** (lines joined):

```
Policy consistently outperforms both diffusion policy and behavior cloning with action chunking. We further perform ablation studies demonstrating that learned score-based selection is significantly more effective than baselines.
```

## Impact on Metrics

1. **Lines ratio**: Our 620 lines vs Gold 1181 → 0.525 ratio → poor Structure score
2. **ROUGE-L**: Long lines break LCS matching at line boundaries
3. **Readability**: Very long lines (>200 chars) vs ~80 char wrapped lines

## Files to Modify

1. `layout/pymupdf_renderer.rs` - Change `join(" ")` to `join("\n")`
