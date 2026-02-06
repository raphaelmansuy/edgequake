# OODA Iteration 23 – Act

## Changes Applied

### Fix: Preserve blank lines in `cleanup_toc_leader_dots()`

**File**: `src/renderers/markdown.rs`, function `cleanup_toc_leader_dots()`

**Before**: All lines that were empty (or became empty after cleaning) were discarded:

```rust
if !cleaned.trim().is_empty() {
    result_lines.push(cleaned);
}
```

**After**: Originally-empty lines are preserved as paragraph separators:

```rust
if line.trim().is_empty() {
    result_lines.push(String::new());
    continue;
}
// ... TOC cleaning logic ...
if !cleaned.trim().is_empty() {
    result_lines.push(cleaned);
}
```

### Debug instrumentation

Temporarily added `RENDER-DEBUG` and `CLEANUP-DEBUG` tracing to isolate which stage discarded blank lines. Traced through: `normalize_excessive_whitespace()` → `join_broken_lines()` → `cleanup_toc_leader_dots()`. Found the bug in the last stage. Removed all debug tracing after fix confirmed.

## Verification

- **569 tests pass** (`cargo test --lib -- --test-threads=4`)
- **84 blank lines preserved** at every pipeline stage
- **Output**: 168 lines (up from 84 lines pre-fix, gold standard is 191 lines — 88% coverage)
- **Clippy**: Only 3 pre-existing warnings

## Commit

Pending — to be committed with this iteration's OODA files.
