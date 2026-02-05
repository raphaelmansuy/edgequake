# OODA Iteration 16 - Decide

## Decision

Implement `join_broken_lines()` function in `markdown.rs` to reconstruct words broken across lines.

## Implementation Plan

### Function: `join_broken_lines(text: &str) -> String`

```rust
/// Join lines that were broken mid-word during PDF extraction.
///
/// WHY: PDF text extraction preserves original text box boundaries,
/// which often break words at the line end. This function reconstructs
/// proper word boundaries for better readability and LLM processing.
///
/// Rules:
/// 1. Join when line ends with lowercase + next starts with lowercase (word split)
/// 2. Remove hyphen when line ends with `word-` + next starts with lowercase
/// 3. Preserve: empty lines, markdown syntax, code blocks, list items
```

### Detection Logic

```
if prev_line ends with [a-z] AND NOT [.!?:;,] AND NOT empty
   AND curr_line starts with [a-z]
   AND prev_line NOT markdown special (# - * | > `)
   AND NOT in code block
then:
   Join prev_line + curr_line (maybe removing trailing hyphen)
```

### Integration Point

Add to `cleanup_markdown_artifacts()` pipeline, BEFORE other cleanups since this affects raw text flow.

## Test Cases

1. `"TCP/IP netw\norking"` → `"TCP/IP networking"`
2. `"sockets\n- based"` → `"sockets-based"`
3. `"end.\nStart"` → `"end.\nStart"` (preserve sentence boundary)
4. `"- Item\ncontinuation"` → preserve (list item)
5. `"```\ncode\n```"` → preserve code block
