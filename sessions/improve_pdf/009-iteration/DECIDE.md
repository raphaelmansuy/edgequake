# DECIDE.md - Iteration 009

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Decision: Single Line Change Using DocumentStats

### Implementation Plan

**Change:** Replace `50.0` with `stats.typical_line_spacing * 2.5`

**Steps:**

1. Import DocumentStats at top of file (already imported from Loop 007)
2. Calculate stats in process() method
3. Replace magic number with adaptive threshold
4. Run tests

### Exact Code Change

**File:** `processor.rs` (line 2694-2743)

**BEFORE:**

```rust
impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // ...
                        let vertical_gap = (next.bbox.y1 - current.bbox.y2).abs();

                        // Allow larger gap for line spacing (up to ~50pt for double-spaced or with margins)
                        if vertical_gap <= 50.0 {  // MAGIC NUMBER
                            if ends_hyph.is_some() && starts_cont {
                                join_with = Some(i + 1);
                            }
                        }
            // ...
        }
        Ok(document)
    }
}
```

**AFTER:**

```rust
impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Calculate stats once for adaptive threshold (First Principles!)
        let stats = DocumentStats::from_document(&document);
        let max_vertical_gap = stats.typical_line_spacing * 2.5;

        for page in &mut document.pages {
            // ...
                        let vertical_gap = (next.bbox.y1 - current.bbox.y2).abs();

                        // Use adaptive threshold based on document's actual line spacing
                        if vertical_gap <= max_vertical_gap {
                            if ends_hyph.is_some() && starts_cont {
                                join_with = Some(i + 1);
                            }
                        }
            // ...
        }
        Ok(document)
    }
}
```

### Acceptance Criteria

✅ **Magic number eliminated:**

- ~~50.0~~ → `stats.typical_line_spacing * 2.5`

✅ **All 113 tests passing**

✅ **Adaptive behavior:**

- Small fonts: smaller threshold
- Large fonts: larger threshold
- Matches BlockMergeProcessor pattern

✅ **No breaking changes**

### Expected Results

**Standard 10pt PDF:**

- Before: 50pt threshold
- After: ~35pt threshold (14pt × 2.5)
- Impact: More precise, slightly stricter

**Large 18pt PDF:**

- Before: 50pt threshold (TOO SMALL)
- After: ~63pt threshold (25pt × 2.5)
- Impact: NOW WORKS - finds more hyphenations

**Small 8pt PDF:**

- Before: 50pt threshold (TOO LARGE)
- After: ~20pt threshold (8pt × 2.5)
- Impact: Fewer false positives

### Rollback Plan

If tests fail:

1. Adjust multiplier: 2.5 → 3.0 or 2.0
2. Add min/max bounds: `max(20.0, min(100.0, stats.typical_line_spacing * 2.5))`
3. Debug specific failing tests

### Implementation Time

**Estimated:** 2 minutes (simple change)
