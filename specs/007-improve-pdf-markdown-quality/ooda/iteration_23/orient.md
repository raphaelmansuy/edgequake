# OODA Iteration 23 – Orient

## Analysis

The `cleanup_toc_leader_dots()` function was designed to remove Table of Contents artifacts (leader dots, standalone page numbers, empty bold markers). However, its "only keep non-empty lines" filter was too aggressive — it treated ALL empty lines as unwanted, including the blank lines that serve as markdown paragraph separators.

### Pipeline flow

```
render_header()  → output += "# **Title**\n\n"     (adds blank line)
render_text()    → output += "paragraph text\n\n"   (adds blank line)
  ...
normalize_excessive_whitespace() → preserves blank lines ✓
cleanup_markdown_artifacts():
  join_broken_lines()           → preserves blank lines ✓
  cleanup_toc_leader_dots()     → REMOVES ALL blank lines ✗  ← BUG
  convert_standalone_bold()     → operates on already-broken output
```

### Why the bug wasn't caught

The existing tests create simple documents with 1-3 blocks. The `\n\n` between blocks doesn't significantly affect short test outputs. The quality tests compare normalized content (split_whitespace), which ignores whitespace differences.

### Fix principle

Empty lines in the input to `cleanup_toc_leader_dots` are NEVER TOC artifacts — they're structural paragraph separators. Only lines that START with content and BECOME empty after cleaning (dot removal, page number removal) should be dropped.

## Options

1. **Preserve empty input lines unconditionally** (chosen)
2. Track whether a line was originally empty vs became empty
3. Skip empty line filtering entirely

## Recommendation

Option 1: Add an early check at the top of the loop — if the line is already empty before any cleaning, preserve it as a blank line separator.
