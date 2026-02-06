# OODA Iteration 39 — Decide

## Fix: Raise Backend Header Threshold, Eliminate Levels 3-4

### Approach

Modify `classify_blocks()` in `pdfium_backend.rs` to only classify headers with high confidence:

**Before:**

```rust
let level = if size_ratio >= 1.8 { 1 }
       else if size_ratio >= 1.5 { 2 }
       else if size_ratio >= 1.3 { 3 }  // ← FALSE POSITIVES
       else { 4 };                       // ← OVER-DEEP HEADERS
```

**After:**

```rust
let level = if size_ratio >= 1.8 { 1 }
       else { 2 };  // All other large text → level 2
// Only classify if ratio >= 1.5 (was 1.2)
```

Also raise the `header_threshold` from `body_size * 1.2` to `body_size * 1.4`:

- Matches `HeadingClassifier`'s conservative threshold
- Eliminates false header classification for 1.2-1.4x text
- Lets downstream processors handle borderline cases

### Expected Impact

1. **False `###` headers eliminated** — no more levels 3 or 4
2. **Real section headers become `##`** — downstream SectionPatternProcessor will classify "1. INTRODUCTION" as H2 instead of backend classifying as H4
3. **Consistent heading depth** — all headers use levels 1-2, matching the downstream processor design

### Risk Assessment

- **Low risk**: The downstream processors already handle header detection well
- **Regression check**: Elitizon doc should not change (its headers are already correct)
- **Test**: Verify all 462 tests still pass

### Files to Modify

1. `src/backend/pdfium_backend.rs` — raise threshold, simplify level assignment
2. Tests in same file — update test expectations
