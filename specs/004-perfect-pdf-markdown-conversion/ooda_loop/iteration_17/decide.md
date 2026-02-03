# OODA-17: Decide Phase

## Decision

Add `merge_consecutive_title_lines()` function to TextGrouper to merge multi-line titles into single blocks.

## Implementation Plan

### Step 1: Add merge function to TextGrouper

```rust
/// Merge consecutive spanning lines that form a single multi-line title.
///
/// # WHY
///
/// PDF titles often wrap across 2-3 lines due to centering and length.
/// Each line is extracted as a separate text object with different Y-coordinates.
/// We need to merge them into a single block to render as one heading.
///
/// # Algorithm
///
/// 1. For each consecutive pair of lines
/// 2. If Y-gap < 1.5 × font_size AND font sizes match within 1pt
/// 3. Merge into single line
/// 4. Otherwise keep separate
fn merge_consecutive_title_lines(
    &self, 
    lines: Vec<Vec<TextElement>>
) -> Vec<Vec<TextElement>>
```

### Step 2: Call it after grouping spanning elements

In `group_two_column_layout()`, after:
```rust
let spanning_lines = self.group_single_column_layout(spanning_elements);
```

Add:
```rust
let spanning_lines = self.merge_consecutive_title_lines(spanning_lines);
```

### Step 3: Also apply to single-column layouts

In `group_single_column_layout()`, the function is called for ALL elements. But we want title merging only for spanning (title zone) elements. So we'll handle this in `group_two_column_layout()` only.

For truly single-column pages, we need to identify title zone elements and merge them. This can be a future enhancement if needed.

## Code Changes

### File: `src/backend/text_grouping.rs`

**New function**: `merge_consecutive_title_lines()`

```rust
fn merge_consecutive_title_lines(
    &self,
    lines: Vec<Vec<TextElement>>,
) -> Vec<Vec<TextElement>> {
    if lines.len() <= 1 {
        return lines;
    }

    let mut result: Vec<Vec<TextElement>> = Vec::new();
    let mut current_line: Vec<TextElement> = Vec::new();
    let mut prev_y: Option<f32> = None;
    let mut prev_font_size: Option<f32> = None;

    for line in lines {
        if line.is_empty() {
            continue;
        }

        // Calculate line properties
        let line_y = line.iter().map(|e| e.y).sum::<f32>() / line.len() as f32;
        let line_font_size = line.iter().map(|e| e.font_size).sum::<f32>() / line.len() as f32;

        let should_merge = if let (Some(py), Some(pf)) = (prev_y, prev_font_size) {
            let y_gap = (line_y - py).abs();
            let font_diff = (line_font_size - pf).abs();
            
            // WHY 1.5 × font_size: Normal title line spacing is 1.2-1.5× font
            // WHY 1pt font tolerance: Handle minor font size variations
            y_gap < line_font_size * 1.5 && font_diff < 1.0
        } else {
            false
        };

        if should_merge {
            // Merge with current line
            current_line.extend(line);
        } else {
            // Start new line
            if !current_line.is_empty() {
                result.push(current_line);
            }
            current_line = line;
        }

        prev_y = Some(line_y);
        prev_font_size = Some(line_font_size);
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    // Log if we merged any lines
    if result.len() < lines.len() {
        tracing::info!(
            "OODA-17: Merged {} title lines into {} blocks",
            lines.len(),
            result.len()
        );
    }

    result
}
```

**Integration point**: In `group_two_column_layout()`, after:
```rust
let spanning_lines = self.group_single_column_layout(spanning_elements);
```

Insert:
```rust
// OODA-17: Merge multi-line titles in spanning zone
let spanning_lines = self.merge_consecutive_title_lines(spanning_lines);
```

## Expected Impact

| Document | Before | Expected After | Reason |
|----------|--------|----------------|--------|
| agent_2510.09244v1 | 80.1% | ~82% | Correct title structure |
| Other multi-line titles | improved | improved | Same fix applies |

## Commit Message

```
OODA-17: Merge multi-line PDF titles into single blocks

- Add merge_consecutive_title_lines() to TextGrouper
- Merge spanning lines with Y-gap < 1.5× font_size
- Fixes "Fundamentals of Building Autonomous LLM" / "Agents" split
- Improves title structure for academic papers with wrapped titles
```
