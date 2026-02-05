# Iteration 05: OBSERVE - Text Span Fragmentation Analysis

**Date:** 2025-02-05
**Focus:** Text fragmentation causing broken paragraphs and isolated style markers

## Observed Issues

### 1. Extracted Output (AI_Services\_\_Elitizon.pdf)

```markdown
# **AI Services - Elitizon**

**Executive summary**

Elitizon designs and delivers production-grade AI systems with a focus on

**workflows** ← ISSUE: "workflows" isolated as separate styled word

teams move from prototypes to reliable, governed deployments...
```

**Problem:** Style spans are becoming separate blocks instead of inline text.

### 2. Root Cause Analysis

The PDF text extraction produces spans with different styles:

```
Span 1: "Elitizon designs... focus on " (regular)
Span 2: "workflows" (bold)
Span 3: " teams move..." (regular)
```

When these spans are processed, the bold word gets separated because:

1. Block boundaries are created at style transitions
2. Reading order reconstruction breaks at style changes
3. The renderer outputs each block on its own line

### 3. PyMuPDF4LLM Comparison

PyMuPDF4LLM handles this correctly:

```python
for i, s in enumerate(spans):  # iterate spans OF THE LINE
    bold = s["flags"] & 16 or s["char_flags"] & 8
    # Apply inline styling without breaking line
    text = f"{prefix}{s['text'].strip()}{suffix} "
    out_string += text  # ← All spans concatenated inline
```

**Key insight:** PyMuPDF4LLM iterates spans _within a line_, not across blocks.

### 4. Current Architecture Gap

```
┌─────────────────────────────────────────────────────────────┐
│  PDF → Backend → Blocks → Renderer → Markdown               │
├─────────────────────────────────────────────────────────────┤
│  ISSUE: Blocks are created at style transitions             │
│         Each block becomes a separate paragraph             │
│                                                             │
│  PDF Font Change → New Block → New Paragraph Line           │
│                                                             │
│  Expected: Font changes → Inline style markers              │
│           "text **bold** more text"                         │
│                                                             │
│  Actual: Font changes → Separate blocks                     │
│          "text"                                             │
│          "**bold**"                                         │
│          "more text"                                        │
└─────────────────────────────────────────────────────────────┘
```

### 5. Files Involved

1. `src/backend/extraction_engine.rs` - Creates elements with font info
2. `src/backend/element_processing.rs` - Groups elements into blocks
3. `src/layout/pymupdf_grouper.rs` - Line and block grouping
4. `src/renderers/markdown.rs` - Output formatting

### 6. Investigation: Block Creation Logic

Need to trace where blocks are split at style boundaries vs. where they should
remain as spans within a single block.
