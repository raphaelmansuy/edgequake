```markdown
# OODA Iteration 18 - Orient

## Analysis

### Why Content Gets Concatenated

In the renderer's `Renderer::render()` method, the final output goes through:

1. `normalize_excessive_whitespace()` - collapses multiple spaces
2. `cleanup_markdown_artifacts()` - cleans artifacts, joins broken lines

The `join_broken_lines()` function is OVER-JOINING in some cases. When content like:
```

traces.\n\nreadiness.

```
passes through, the `\n\n` should be preserved as a paragraph break. But the cleanup pipeline removes it.

### The Real Problem: `normalize_excessive_whitespace()`

Looking at the function, it removes excess spaces within lines. But it may also be collapsing paragraph boundaries.

### Strategy: Add Sentence-Boundary Paragraph Splitting

After all cleanup, add a final pass that:
1. Detects sentences that run together without proper separation
2. Adds paragraph breaks between them when they appear to be from different topics

### Implementation: `split_concatenated_paragraphs()`

```

Input: "traces. readiness."  
Output: "traces.\n\nreadiness."

Input: "policy. Pragmatic automation: autonomous"
Output: "policy.\n\nPragmatic automation: autonomous"

```

Rules:
- If a line contains ". " followed by an uppercase letter, and the segment before
  and after are both substantial (>30 chars), insert a paragraph break
- Preserve intentional inline periods (abbreviations like "e.g.", "Dr.", "etc.")
- Don't split within quotes or parentheses
```
