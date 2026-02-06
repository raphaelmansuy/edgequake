````markdown
# OODA Iteration 17 - Decide

## Decision

Implement **paragraph continuation detection** in the page renderer to join consecutive Text blocks that are part of the same paragraph.

## Implementation Plan

### 1. Add `is_paragraph_continuation()` helper to MarkdownRenderer

```rust
/// Detect if block B is a paragraph continuation of block A.
/// WHY: PDF backends extract bold/styled text as separate blocks,
/// fragmenting paragraphs like "focus on **workflows** teams".
fn is_paragraph_continuation(prev: &Block, curr: &Block) -> bool
```
````

### 2. Modify `render_page_with_arxiv()` to use continuation detection

Instead of always adding `\n\n` after render_text, check if the next block is a continuation:

- If YES: add just a space (or nothing for inline bold)
- If NO: add `\n\n` as before

### 3. Update `render_text()` to support continuation mode

Add a parameter or flag to control whether `\n\n` is appended.

### Integration

```
render_page_with_arxiv()
  for (i, block) in page.blocks.iter().enumerate():
    if is_continuation(prev_block, block):
      render_text_inline(block, output)  // No \n\n
    else:
      render_block(block, output)        // Normal with \n\n
```

## Test Cases

1. "focus on" + "workflows" + "teams move" → "focus on **workflows** teams move"
2. "sentence ends." + "New paragraph" → separate paragraphs
3. "header text" + "body text" → separate (different block types)
4. List items remain separate
5. Code blocks remain separate

```

```
