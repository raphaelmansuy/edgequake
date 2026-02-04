# OODA-02: Orient

## Mission Re-Read ✅

**File**: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
**Goal**: F1 >= 0.95 by implementing pymupdf4llm algorithms

---

## Analysis Framework

I will analyze each missing algorithm by:

1. **Impact**: Expected F1 improvement
2. **Effort**: Implementation complexity
3. **Risk**: Chance of introducing regressions

---

## Algorithm Impact Analysis

### 1. Filter Rotated/Vertical Text

**Impact**: HIGH (0.03-0.05 F1 gain expected)

**Rationale**: The arXiv sidebar text (dates, IDs) is the #1 source of garbage:

```
5 2 0 2 c e D 1 3  ← "31 Dec 2025" rotated
```

**pymupdf4llm Algorithm**:

```python
if abs(1 - line_dir[0]) > 1e-3:  # dir[0] is cos(angle)
    continue  # skip non-horizontal
```

**Implementation**:

- PDFium provides text bounds via `PdfRect`
- For each character, we can compute the angle from bounding box orientation
- Characters with width < height AND rotated bbox → likely vertical text

**Effort**: LOW (10 lines of code)
**Risk**: LOW (only filters, doesn't change grouping)

---

### 2. Word Join Threshold (10% vs 25%)

**Impact**: MEDIUM (0.01-0.02 F1 gain expected)

**Rationale**: Current 25% threshold is too aggressive, merging separate words.

**pymupdf4llm Algorithm**:

```python
delta = s1["size"] * 0.1  # 10% of font size
if s0["bbox"].x1 + delta < s1["bbox"].x0:
    continue  # don't join - gap too large
```

**Current Implementation** (`pymupdf_structs.rs`):

```rust
let space_width = self.font_size * 0.25;  // ← Should be 0.10
```

**Implementation**: Change constant from 0.25 to 0.10

**Effort**: TRIVIAL (1 line change)
**Risk**: LOW (may require tuning)

---

### 3. Block Vertical Gap (10pt vs 20pt)

**Impact**: MEDIUM (0.02-0.03 F1 gain expected)

**Rationale**: 20pt gap is too aggressive, merging unrelated paragraphs. 10pt is the pymupdf4llm standard.

**Current Implementation** (`pymupdf_grouper.rs`):

```rust
const DEFAULT_BLOCK_GAP: f32 = 20.0;  // ← Should be 10.0
```

**Effort**: TRIVIAL (1 line change)
**Risk**: LOW

---

### 4. Boundary Normalization (Phase 2)

**Impact**: HIGH (0.03-0.05 F1 gain expected)

**Rationale**: AlphaEvolve has 4x fragmentation because blocks aren't being merged when they should be. Phase 2 normalizes x0/x1 boundaries to within 3pt, then merges vertically.

**pymupdf4llm Algorithm**:

```python
# Normalize left boundary
x0 = min([bb.x0 for bb in prects if abs(bb.x0 - b.x0) <= 3])
x1 = max([bb.x1 for bb in prects if abs(bb.x1 - b.x1) <= 3])

# Join if aligned and close
if abs(r.x0 - r0.x0) <= 3 and abs(r.x1 - r0.x1) <= 3 and abs(r0.y1 - r.y0) <= 10:
    r0 |= r  # merge
```

**Implementation**: New function `join_blocks_phase2()`

**Effort**: MEDIUM (30-40 lines)
**Risk**: MEDIUM (may over-merge if not careful)

---

### 5. Smart Sort Key (Phase 3)

**Impact**: HIGH (0.02-0.04 F1 gain expected)

**Rationale**: Multi-column reading order is wrong. The smart sort key ensures left column is read before right column even when they overlap vertically.

**pymupdf4llm Algorithm**:

```
       Q +---------+    For block Q:
         | next is |    1. Find P = left-most block with vertical overlap
   P +-------+   |  |   2. Sort key = (P.y0, Q.x0)
     | left  |   |  |   3. This ensures Q comes after P
     | block |   +--+
     +-------+
```

**Current Implementation**: Uses center-based column detection which fails when columns don't align perfectly.

**Effort**: MEDIUM (40-50 lines)
**Risk**: LOW (sorting only, doesn't change content)

---

### 6. Vertical Join (Phase 1)

**Impact**: MEDIUM (0.01-0.02 F1 gain expected)

**Rationale**: Joins rectangles that "touch" each other with 10pt vertical gap tolerance.

**pymupdf4llm Algorithm**:

```python
delta = (0, 0, 0, 10)  # allow 10pt gap below
while prects:
    prect0 = prects[0]
    for i in range(len(prects)-1, 0, -1):
        if ((prect0 + delta) & prects[i]).is_valid:  # intersection exists
            prect0 |= prects[i]  # merge
```

**Effort**: LOW (20 lines)
**Risk**: LOW

---

## Priority Matrix

```
Impact ▲
       │
  HIGH │  [1: Filter rotated] [4: Boundary norm] [5: Smart sort]
       │
MEDIUM │  [2: Word join 10%] [3: Block gap 10pt] [6: Vertical join]
       │
  LOW  │
       │
       └──────────────────────────────────────────────────────────▶
           TRIVIAL          LOW            MEDIUM          Effort
```

---

## Recommended Priority Order

| Priority | Algorithm           | Expected F1 Gain | Cumulative | Effort  |
| -------- | ------------------- | ---------------- | ---------- | ------- |
| P0       | Filter rotated text | +0.04            | 0.91       | LOW     |
| P1       | Word join 10%       | +0.015           | 0.925      | TRIVIAL |
| P2       | Block gap 10pt      | +0.02            | 0.945      | TRIVIAL |
| P3       | Boundary normalize  | +0.03            | 0.975      | MEDIUM  |
| P4       | Smart sort key      | +0.02            | 0.995      | MEDIUM  |

**Total Expected**: F1 = 0.97+ (exceeds 0.95 target)

---

## Risk Mitigation

1. **Test after each change**: Run F1 evaluation after every algorithm change
2. **Commit incrementally**: One commit per algorithm for easy rollback
3. **Keep constants configurable**: Use struct parameters, not hardcoded values
4. **Document WHY**: Add comments explaining pymupdf4llm algorithm origin

---

## First Principles Validation

The pymupdf4llm algorithms are based on these first principles:

1. **Text on same line**: Spans with same baseline (within 3pt tolerance)
2. **Words in same span**: Gap < 10% of font size
3. **Paragraphs in same block**: Same x0/x1 boundaries (within 3pt), vertical gap < 10pt
4. **Reading order**: Left column before right, top before bottom
5. **Ignore noise**: Rotated text, white text, invisible text

These principles are derived from how humans read documents. They are stable across different PDF generators and layout tools.

---

## Next: Decide

Create specific implementation plan with file:line targets.
