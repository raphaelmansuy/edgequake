# OODA Iteration 10 – Decide

**Date:** 2026-02-07

## Decisions

1. Add `page_separators: bool` to `MarkdownConfig` (default: true)
2. When enabled: `\n-----\n\nPage N\n\n` between pages (N is 1-indexed)
3. When disabled: plain `\n---\n\n` (previous behavior)
4. Add 2 tests: enabled and disabled modes
