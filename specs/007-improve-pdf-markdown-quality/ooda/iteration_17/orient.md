````markdown
# OODA Iteration 17 - Orient

## Analysis

### Why BlockMergeProcessor Can't Fix This

BlockMergeProcessor rejects merges when font weight differs:

```rust
let weight_a = span_a.style.weight.unwrap_or(400);
let weight_b = span_b.style.weight.unwrap_or(400);
if (weight_a >= 600) != (weight_b >= 600) {
    return false; // Different weights → reject
}
```
````

This is CORRECT behavior for preventing header-text merges. But it also prevents inline bold (e.g., "with a focus on **workflows** teams move from") from being recognized.

### The Real Fix: Paragraph Continuation Detection in Renderer

The renderer's `render_page_with_arxiv()` processes blocks sequentially:

```
for block in page.blocks:
    render_block(block, output)  // Each block adds \n\n
```

We need to detect when consecutive Text blocks are **continuations** of the same paragraph and render them with just a space instead of `\n\n`.

### Continuation Heuristics

A block B is a continuation of previous block A when:

1. Both are `BlockType::Text` or `BlockType::Paragraph`
2. A does NOT end with sentence-ending punctuation (. ! ? :)
3. B starts with lowercase letter OR B is very short (single bold word)
4. B is NOT a known structural element (header-like, list prefix)
5. Both blocks are on the same page
6. Vertical gap between A and B is small (within line spacing)

### PyMuPDF4LLM Approach

PyMuPDF4LLM handles this at the line-merging level in `get_text_lines.py`:

- Groups characters into lines based on Y-coordinate
- Merges lines within the same paragraph using indentation and spacing
- Bold/italic characters are marked inline with markdown syntax

Our approach must work post-extraction since we can't change the backend.

### Risk

- May incorrectly join blocks that are separate paragraphs
- Must preserve paragraph boundaries where spacing is intentional

### Mitigation

- Only join when both blocks are Text type
- Require small vertical gap (use block bbox)
- Never join across page boundaries

```

```
