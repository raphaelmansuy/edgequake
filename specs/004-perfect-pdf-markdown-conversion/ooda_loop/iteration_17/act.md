# OODA-17: Act Phase

## Implementation Summary

**Problem**: Multi-line PDF titles were being split into separate heading blocks, causing incorrect markdown output like:

```markdown
# Fundamentals of Building Autonomous LLM

## Agents
```

**Solution**: Added `merge_consecutive_title_lines()` function to merge title lines that are:

- Close in Y-spacing (< 1.5 × font_size)
- Same font size (within 1pt tolerance)

## Code Changes

### File: `src/backend/text_grouping.rs`

**New function** (lines ~720-785):

```rust
fn merge_consecutive_title_lines(
    &self,
    lines: Vec<Vec<TextElement>>,
) -> Vec<Vec<TextElement>> {
    // ... merges consecutive lines with:
    // - Y-gap < 1.5 × font_size (normal title spacing)
    // - Font size match within 1pt
}
```

**Integration** (line ~369):

```rust
// Process spanning elements first (titles, etc.)
let spanning_lines = self.group_single_column_layout(spanning_elements);

// OODA-17: Merge multi-line titles in spanning zone
let spanning_lines = self.merge_consecutive_title_lines(spanning_lines);
```

## Verification

**Before OODA-17**:

```markdown
# Fundamentals of Building Autonomous LLM

## Agents
```

**After OODA-17**:

```markdown
# Fundamentals of Building Autonomous LLM Agents
```

**Log output**:

```
OODA-17: Merged 2 title lines into 1 blocks
```

## Quality Metrics

No change in automated quality scores (86.5%) because:

- Word overlap scores don't distinguish single vs multi-line titles
- Structure score counts headings, but "# Title" + "## Agents" has same count as "# Title Agents"

However, the output quality is significantly improved for human readability.

## Commit

```
OODA-17: Merge multi-line PDF titles into single blocks

- Add merge_consecutive_title_lines() to TextGrouper
- Merge spanning lines with Y-gap < 1.5× font_size
- Fixes "Fundamentals of Building Autonomous LLM" / "Agents" split
- Improves title formatting for academic papers with wrapped titles
- Quality: 86.5% (maintained)
```
