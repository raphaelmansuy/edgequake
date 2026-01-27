# OBSERVE.md - Iteration 009

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Target: HyphenContinuationProcessor Magic Number

### Current State Analysis

#### File: `processor.rs` (lines 2530-2833)

```rust
pub struct HyphenContinuationProcessor {}

impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let mut i = 0;
            while i < page.blocks.len() {
                if i + 1 < page.blocks.len() {
                    let current = &page.blocks[i];
                    let next = &page.blocks[i + 1];

                    if current.block_type == BlockType::Text && next.block_type == BlockType::Text {
                        let vertical_gap = (next.bbox.y1 - current.bbox.y2).abs();

                        let ends_hyph = self.ends_with_hyphen(&current.text);
                        let starts_cont = self.starts_with_continuation(&next.text);

                        // ❌ MAGIC NUMBER: 50.0
                        // Allow larger gap for line spacing (up to ~50pt for double-spaced or with margins)
                        if vertical_gap <= 50.0 {  // Line 2743
                            if ends_hyph.is_some() && starts_cont {
                                tracing::debug!("Immediate hyphen join triggered");
                                join_with = Some(i + 1);
                            }
                        }
                    }
                }
                // ...
            }
        }
        Ok(document)
    }
}
```

### Magic Number Identified

| Magic Number | Purpose                                  | Context                | First Principles Violation                        |
| ------------ | ---------------------------------------- | ---------------------- | ------------------------------------------------- |
| **50.0**     | Max vertical gap for hyphen continuation | Line spacing tolerance | Should be based on document's actual line spacing |

### Root Cause: Fixed Pixel Value

**Problem:** 50.0pt doesn't adapt to:

1. **Font sizes:** 8pt documents need smaller gap tolerance, 14pt need larger
2. **Line spacing:** Single-spaced vs double-spaced documents
3. **Document styles:** Academic papers vs technical manuals

### First Principles Analysis

#### Typography Fundamentals

1. **Line Spacing = Body Font × Leading Factor**

   - Single-spaced: body_size × 1.2
   - Normal spacing: body_size × 1.4-1.5
   - Double-spaced: body_size × 2.0
   - Academic papers: typically 1.4-1.5x

2. **Hyphen Continuation Should Allow:**
   - Same line: gap = 0 (immediate adjacency)
   - Next line: gap ≤ line_spacing × 2.0 (allow some margin)
   - Double-spaced: gap ≤ line_spacing × 2.5 (larger tolerance)

### Current Behavior Analysis

**Fixed 50pt Vertical Gap:**

- 10pt font (14pt spacing): 50/14 = 3.6x ✓ Reasonable
- 12pt font (17pt spacing): 50/17 = 2.9x ✓ Reasonable
- 8pt font (11pt spacing): 50/11 = 4.5x ❌ Too large (might join unrelated lines)
- 18pt font (25pt spacing): 50/25 = 2.0x ✓ Works but could be more precise
- 24pt font (34pt spacing): 50/34 = 1.5x ❌ Too small (misses valid hyphenations)

### Solution: Use DocumentStats

**Loop 007 already provides:** `DocumentStats.typical_line_spacing`

This is the PERFECT value to derive from!

```rust
// Adaptive threshold based on actual document line spacing
let max_gap = stats.typical_line_spacing * 2.5;

if vertical_gap <= max_gap {
    // Join hyphenated words
}
```

**Rationale for 2.5x:**

- Covers single-spaced (1.2x) to near double-spaced (2.0x)
- Allows some paragraph spacing tolerance
- Matches BlockMergeProcessor's approach (consistency!)

### Comparison: Before vs After

#### Before (Magic Number)

```rust
if vertical_gap <= 50.0 {  // Fixed threshold
    // Join blocks
}
```

**Problems:**

- Doesn't scale with font size
- Too large for small fonts (false positives)
- Too small for large fonts (false negatives)

#### After (Adaptive)

```rust
let max_gap = stats.typical_line_spacing * 2.5;
if vertical_gap <= max_gap {  // Adaptive threshold
    // Join blocks
}
```

**Advantages:**

- Scales with document's actual line spacing
- Works on any font size
- Consistent with BlockMergeProcessor approach

### Examples

**10pt Font (typical_line_spacing = 14pt):**

- Threshold: 14 × 2.5 = 35pt (current: 50pt)
- More precise, reduces false positives

**12pt Font (typical_line_spacing = 17pt):**

- Threshold: 17 × 2.5 = 42.5pt (current: 50pt)
- Similar behavior

**18pt Font (typical_line_spacing = 25pt):**

- Threshold: 25 × 2.5 = 62.5pt (current: 50pt)
- NOW WORKS! (was missing valid hyphenations)

**24pt Font (typical_line_spacing = 34pt):**

- Threshold: 34 × 2.5 = 85pt (current: 50pt)
- NOW WORKS! (was missing many hyphenations)

### Implementation Complexity

**Very Low - Similar to Loop 007:**

- Use existing DocumentStats from Loop 007
- Calculate once in process() method
- Pass as parameter to hyphen detection logic
- Single line change: `50.0` → `stats.typical_line_spacing * 2.5`

### Test Cases to Verify

1. **Standard 10pt PDF:** Similar behavior (35pt vs 50pt)
2. **Large 18pt PDF:** Better hyphen detection (62.5pt vs 50pt)
3. **Small 8pt PDF:** Less false positives (20pt vs 50pt)
4. **Double-spaced:** Works correctly (2.5x multiplier)

### Acceptance Criteria

✅ Zero magic numbers in HyphenContinuationProcessor
✅ Vertical gap threshold adaptive to line spacing
✅ All 113 tests passing
✅ No false positives/negatives on different font sizes
✅ Consistent with BlockMergeProcessor approach

### Risk Assessment

**Very Low Risk:**

- Simple change (one line)
- Non-critical processor (hyphen joining)
- If wrong, words remain hyphenated (minor)
- Tests will catch regressions

### Related Work

**Loop 007:** BlockMergeProcessor uses `stats.typical_line_spacing * 2.5` for same purpose
**Consistency:** Using same formula = predictable, maintainable behavior

### Expected Impact

- **Large Font PDFs:** +5-10 points (better hyphen joining)
- **Small Font PDFs:** +3-5 points (fewer false joins)
- **Robustness:** ++ (works on any font size)
- **Consistency:** ++ (matches BlockMergeProcessor pattern)
