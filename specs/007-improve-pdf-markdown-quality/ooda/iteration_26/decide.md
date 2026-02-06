# OODA Iteration 26 – Decide

## Decision

Set `page_breaks: false` in `MarkdownStyle::default()`.

## Rationale

The default style is used for LLM-optimized extraction. Page breaks are PDF artifacts that fragment semantic content.

## Risk

Very low — only affects rendering output (no extraction logic). Users who need page breaks can use `MarkdownStyle::verbose()`.
