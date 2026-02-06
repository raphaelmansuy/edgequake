# OODA Iteration 23 – Observe

## What happened

After OODA-IT22 fixed word spacing, the markdown output had proper text content but **no blank lines between blocks**. All paragraphs, headings, and sections were rendered as consecutive lines without paragraph separators.

## Evidence

Using `xxd` on the output file:
```
00000000: 2320 2a2a 456c 6974 697a 6f6e 2a2a 0a2a  # **Elitizon**.*
```
Offset `0x0e`: only one `0a` (newline) after "Elitizon**", immediately followed by the next block. No `0a0a` (blank line) present.

Using debug tracing in the `render()` pipeline:
```
Before normalize: 84 blank lines ✓
After normalize:  84 blank lines ✓
After join_broken_lines: 84 blank lines ✓
After cleanup_toc_leader_dots: 0 blank lines ✗ ← BUG
After convert_standalone_bold: 0 blank lines ✗
Final output: 0 blank lines ✗
```

## Root cause

`cleanup_toc_leader_dots()` at line ~1221 had this filter:

```rust
// Only keep non-empty lines
if !cleaned.trim().is_empty() {
    result_lines.push(cleaned);
}
```

This discards ALL empty lines, including blank lines that serve as markdown paragraph separators. Every `\n\n` block separator (where the empty line between them has `"".trim().is_empty() == true`) gets filtered out.

Then `result_lines.join("\n")` joins everything with single newlines, destroying all 84 paragraph breaks in a 168-line document.

## Impact

Without blank lines, the entire markdown document renders as a single paragraph in any markdown viewer. Headers, paragraphs, lists — everything merges together.

## Metrics

- Before fix: 0 blank lines in output (84 lines, all content)
- After fix: 84 blank lines in output (168 lines, properly separated)
- Line count now 168 vs gold standard's 191 (87% coverage)
