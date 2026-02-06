# OODA Iteration 26 – Orient

## Analysis

Page breaks (`---`) are PDF-specific artifacts:

```
┌─────────────────────────────────────────────────────┐
│  PDF pages are physical layout units.               │
│  Markdown is semantic — no concept of "pages".      │
│  Inserting `---` between pages fragments content.   │
│                                                     │
│  For LLM consumption:                               │
│  - Continuous flow > page-level chunks              │
│  - Context window = entire document                 │
│  - Page breaks = noise, not signal                  │
└─────────────────────────────────────────────────────┘
```

## Style hierarchy

- `Default` style: for LLM-optimized extraction → `page_breaks: false`
- `Verbose` style: for debugging/inspection → `page_breaks: true`
- `Minimal` style: already has `page_breaks: false`

## Fix

Change `MarkdownStyle::default()` to set `page_breaks: false`.
