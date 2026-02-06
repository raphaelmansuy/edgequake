# OODA Iteration 23 – Decide

## Decision

Fix `cleanup_toc_leader_dots()` in `src/renderers/markdown.rs` to preserve empty lines that serve as markdown paragraph separators.

## Change

Add an early check at the start of the line processing loop:

```rust
// WHY preserve empty lines: Empty lines are markdown paragraph separators.
// Discarding them collapses all blocks into a single paragraph.
// Only CONTENT lines that become empty after cleaning should be skipped.
if line.trim().is_empty() {
    result_lines.push(String::new());
    continue;
}
```

This ensures:

1. Originally-empty lines → preserved as separators
2. Content lines that become empty after TOC cleaning → skipped (as intended)
3. Content lines with remaining text after cleaning → preserved

## Files modified

- `src/renderers/markdown.rs`: `cleanup_toc_leader_dots()` function

## Risk assessment

- Very low risk: only adds a guard for empty lines before existing cleaning logic
- 569 tests pass
- Visual verification confirms proper blank lines in output
