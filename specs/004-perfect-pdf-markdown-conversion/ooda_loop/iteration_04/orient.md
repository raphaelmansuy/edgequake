# OODA Iteration 04 - Orient

## First Principles Analysis

### PDF Coordinate Systems

PDFs use a coordinate system where:

- **Y=0 at bottom** of page (like a graph)
- **Y increases upward** (opposite of document reading order)

Documents should be read:

- **Top to bottom** (decreasing PDF Y)
- **Left to right** (increasing X)

### Current Normalization Logic

The extraction engine has two branches:

1. **Flipped PDFs** (`is_flipped = true`):
   - Detected when Y range > 1.5× page height
   - Normalization: `normalized_y = max_y - y`
   - Result: Y=0 at **top** of page ✓

2. **Normal PDFs** (`is_flipped = false`):
   - Most PDFs fall here
   - Normalization: `normalized_y = y - min_y`
   - Result: Y=0 at **bottom** of page ✗

### Why This Matters

The `text_grouping.rs` and `reading_order.rs` modules **assume** Y=0 is at the top:

- They sort ascending Y → lower Y blocks first
- If Y=0 is at bottom, this produces **reversed reading order**

### Impact on Test PDFs

| PDF                  | is_flipped | Y Normalization | Reading Order |
| -------------------- | ---------- | --------------- | ------------- |
| Qwen.pdf             | true       | max_y - y       | ✓ Correct     |
| hotmess\_\*.pdf      | false      | y - min_y       | ✗ Reversed    |
| agentfail\_\*.pdf    | false      | y - min_y       | ✗ Reversed    |
| Apple-Sandbox-\*.pdf | false      | y - min_y       | ✗ Reversed    |

## Options Analysis

### Option A: Fix normalization in extraction_engine.rs

**Approach**: For normal PDFs, normalize to `max_y - y` instead of `y - min_y`.

```rust
// Before (wrong)
e.y -= min_y;

// After (correct)
e.y = max_y - e.y;  // Flip Y so Y=0 is at top
```

**Pros**:

- Fixes root cause at source
- All downstream code works correctly
- Matches the existing logic for flipped PDFs

**Cons**:

- May affect existing tests (but they're wrong anyway)
- Need to verify all PDFs still work

**Risk**: Medium - need regression testing

### Option B: Reverse sort order in text_grouping.rs

**Approach**: Sort descending Y for normal PDFs.

**Cons**:

- Requires passing `is_flipped` flag through multiple layers
- Inconsistent coordinate system throughout codebase
- More complex maintenance

**Risk**: High - complexity

### Option C: Invert Y in reading_order.rs

**Approach**: Sort descending Y instead of ascending.

**Cons**:

- Breaks flipped PDFs (which currently work)
- Inconsistent behavior

**Risk**: High - breaks working code

## Recommendation

**Option A** is the correct approach:

1. Normalize ALL PDFs to have Y=0 at top
2. Makes coordinates consistent throughout codebase
3. Existing downstream sorting logic remains correct

## Additional Observations

### Text Fragmentation Root Cause

The fragmentation ("failuresbe trained") is a **separate issue**:

- Happens during line grouping or block merging
- Lines from same paragraph concatenated without space
- Need to investigate `group_into_lines` and `BlockMergeProcessor`

### TOC Corruption Root Cause

The "55555..." pattern in Apple Sandbox Guide suggests:

- Possibly glyph substitution issues with special fonts
- Or ToUnicode CMap mapping failures
- Need separate investigation

## Priority Order

1. **P1**: Fix Y normalization (affects all non-flipped PDFs)
2. **P2**: Fix text fragmentation (affects readability)
3. **P3**: Fix TOC/glyph corruption (affects specific PDFs)
4. **P4**: Formula detection (specialized content)
