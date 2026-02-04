# OODA-25 Observe: Figure Caption Handling

## Mission Refresh

Re-read specs/004-perfect-pdf-markdown-conversion.md at start of iteration.

## Current State

### OODA-24 Results

- Superscript digit filtering implemented
- Standalone "1", "2", "3" markers now filtered
- 415 lib tests pass

### Remaining Issues in one_tool PDF

Looking at the current output:

```
14: *Figure 1.Illustration of a LLM navigating through a code reposi-*
15:
16: tory. The LLM is equipped with a single yet powerful tool:jump...
```

The Figure caption:

1. Appears inline in the text flow (interrupts paragraph)
2. Has hyphenation issues ("reposi-" + "tory")
3. Should be formatted as blockquote per gold file

Gold file format:

```
> **Figure 1 Description:** An illustration of an LLM navigating...
```

## Analysis

### Figure Caption Current Behavior

1. Figure caption is detected and rendered as italic (`*...*`)
2. Appears inline because reading order places it by Y position
3. Caption continues on next line due to hyphenation

### Figure Caption Issues

1. **Position**: Should come AFTER current paragraph, not in middle
2. **Format**: Should be blockquote, not italics
3. **Hyphenation**: "reposi-" + "tory" should merge

### Existing Figure Detection Code

Let me check the current figure/caption detection:

- `structure_detection.rs` - HeaderDetectionProcessor
- `processor.rs` - StyleDetectionProcessor
- `markdown.rs` - render_caption() and render_figure()

## Files to Investigate

1. src/processors/structure_detection.rs
2. src/renderers/markdown.rs
3. src/layout/reading_order.rs
