# OODA Iteration 10 – Orient

**Date:** 2026-02-07

## Analysis

Page separators with page numbers are useful for:

- LLM context: knowing which page content comes from
- Document navigation
- Debugging extraction issues

Format choice: `-----` followed by `Page N` on next line. This is:

- Markdown-compatible (horizontal rule)
- Visually clear
- Easy to parse programmatically
- Configurable (can be disabled for cleaner output)
