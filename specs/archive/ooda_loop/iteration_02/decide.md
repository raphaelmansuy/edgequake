# OODA-02: Decide

## Mission Re-Read ✅

**File**: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
**Goal**: F1 >= 0.95

---

## Decision Statement

**DECISION**: Implement 5 algorithm improvements in priority order, testing after each:

1. **P0**: Filter rotated/vertical text (highest impact)
2. **P1**: Reduce word join threshold to 10%
3. **P2**: Reduce block gap to 10pt
4. **P3**: Add boundary normalization (Phase 2)
5. **P4**: Implement smart sort key (Phase 3)

---

## Implementation Plan

### Step 1: Filter Rotated Text [P0]

**File**: `src/layout/pymupdf_grouper.rs`
**Location**: `chars_to_spans()` function

**Algorithm**:

```rust
// Filter out rotated characters based on bbox aspect ratio
// Vertical text has height >> width for each character
fn is_horizontal_char(ch: &RawChar) -> bool {
    let width = ch.x1 - ch.x0;
    let height = ch.y1 - ch.y0;
    // For horizontal text: width is typically similar to or greater than height
    // For vertical text: height >> width (often 3-10x)
    // Use ratio threshold of 2.0 - if height > 2 * width, likely vertical
    width > 0.1 && (height / width) < 2.5
}
```

**Why**:

- arXiv margin dates are vertical text (each char stacked)
- Their bbox has height 10-12pt but width only 4-6pt
- Filtering these removes the scattered "5 2 0 2 c e D 1 3" garbage

**Test**: Run F1, expect ~0.04 improvement

---

### Step 2: Reduce Word Join Threshold [P1]

**File**: `src/layout/pymupdf_structs.rs`
**Location**: `Span::can_append()` method, line ~97

**Current**:

```rust
let space_width = self.font_size * 0.25;  // ← TOO AGGRESSIVE
```

**Change to**:

```rust
let space_width = self.font_size * 0.10;  // pymupdf4llm standard
```

**Why**: pymupdf4llm uses 10% threshold (line 81-82 of get_text_lines.py)

**Test**: Run F1, expect ~0.015 improvement

---

### Step 3: Reduce Block Gap [P2]

**File**: `src/layout/pymupdf_grouper.rs`
**Location**: `GroupingParams` struct

**Current**:

```rust
pub block_gap: f32 = 20.0,
```

**Change to**:

```rust
pub block_gap: f32 = 10.0,  // pymupdf4llm standard
```

**Why**: 10pt is the pymupdf4llm standard (multi_column.py line 242)

**Test**: Run F1, expect ~0.02 improvement

---

### Step 4: Boundary Normalization (Phase 2) [P3]

**File**: `src/layout/pymupdf_grouper.rs`
**New function**: `join_blocks_phase2()`

**Algorithm** (from multi_column.py lines 213-245):

```rust
fn join_blocks_phase2(blocks: &mut Vec<Block>) {
    const TOLERANCE: f32 = 3.0;  // 3pt boundary tolerance
    const VERTICAL_GAP: f32 = 10.0;  // Maximum vertical gap to merge

    // Phase 2a: Normalize x0/x1 boundaries
    for block in blocks.iter_mut() {
        // Find min x0 among blocks with similar x0
        let min_x0 = blocks.iter()
            .filter(|b| (b.x0 - block.x0).abs() <= TOLERANCE)
            .map(|b| b.x0)
            .fold(f32::MAX, f32::min);

        // Find max x1 among blocks with similar x1
        let max_x1 = blocks.iter()
            .filter(|b| (b.x1 - block.x1).abs() <= TOLERANCE)
            .map(|b| b.x1)
            .fold(f32::MIN, f32::max);

        block.x0 = min_x0;
        block.x1 = max_x1;
    }

    // Sort by (x0, y0)
    blocks.sort_by(|a, b| {
        a.x0.partial_cmp(&b.x0).unwrap()
            .then(a.y1.partial_cmp(&b.y1).unwrap().reverse())
    });

    // Phase 2b: Merge blocks with similar boundaries and close Y
    let mut i = 0;
    while i < blocks.len() - 1 {
        let (left, right) = blocks.split_at_mut(i + 1);
        let current = &mut left[i];
        let next = &right[0];

        if (current.x0 - next.x0).abs() <= TOLERANCE
            && (current.x1 - next.x1).abs() <= TOLERANCE
            && (current.y0 - next.y1).abs() <= VERTICAL_GAP
        {
            // Merge next into current
            current.lines.extend(next.lines.clone());
            current.y0 = current.y0.min(next.y0);
            current.y1 = current.y1.max(next.y1);
            blocks.remove(i + 1);
        } else {
            i += 1;
        }
    }
}
```

**Integration point**: Call after `lines_to_blocks()` in the grouping pipeline

**Test**: Run F1, expect ~0.03 improvement

---

### Step 5: Smart Sort Key (Phase 3) [P4]

**File**: `src/layout/pymupdf_grouper.rs`
**Update function**: `sort_blocks_reading_order()`

**Algorithm** (from multi_column.py lines 283-305):

```rust
fn compute_smart_sort_key(&self, block_idx: usize, blocks: &[Block]) -> (f32, f32) {
    let block = &blocks[block_idx];

    // Find left-most block with vertical overlap
    let left_blocks: Vec<_> = blocks.iter()
        .filter(|b| {
            // Must be to the left
            b.x1 < block.x0 &&
            // Must have vertical overlap with our block
            (block.y0 <= b.y0 && b.y0 <= block.y1) ||
            (block.y0 <= b.y1 && b.y1 <= block.y1) ||
            (b.y0 <= block.y0 && block.y1 <= b.y1)
        })
        .collect();

    if let Some(left_block) = left_blocks.iter()
        .max_by(|a, b| a.x1.partial_cmp(&b.x1).unwrap())  // right-most of left blocks
    {
        (left_block.y0, block.x0)  // Use left block's Y
    } else {
        (block.y0, block.x0)  // Use own Y
    }
}
```

**Replace current**: The current `sort_blocks_reading_order()` uses center-based detection

**Test**: Run F1, expect ~0.02 improvement

---

## Success Criteria

| Step | Expected F1 | Cumulative | Status  |
| ---- | ----------- | ---------- | ------- |
| Base | 0.871       | 0.871      | Current |
| P0   | +0.04       | 0.91       | Pending |
| P1   | +0.015      | 0.925      | Pending |
| P2   | +0.02       | 0.945      | Pending |
| P3   | +0.03       | 0.975      | Pending |
| P4   | +0.02       | 0.995      | Pending |

**Target**: F1 >= 0.95 after P2 or P3

---

## Commit Strategy

After each step, commit with message format:

```
feat(pdf): OODA-02-Px: <description>

- <specific change>
- F1: <before> → <after>
```

---

## Risk Mitigation

1. **Run F1 after each step**: Immediately detect regressions
2. **Keep git clean**: Easy rollback if needed
3. **Test edge cases**: Use all 7 gold standard files
4. **Document WHY**: Add comments explaining algorithm origin

---

## Next: Act

Implement in priority order: P0 → P1 → P2 → P3 → P4
