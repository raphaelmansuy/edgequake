# OODA Iteration 24 – Orient

## Root cause analysis: Bold wrapping in headers

### Two code paths produce bold-wrapped headers

1. **`render_header()`** (line ~272): Explicitly formatted as `format!("{} **{}**\n\n", prefix, text.trim())`. The WHY comment claimed "pymupdf4llm wraps all header text in bold markers" — but the gold standard proves this claim FALSE.

2. **`convert_standalone_bold_to_headers()`** (line ~1241): When converting standalone `**text**` to header, it emitted `## **text**` — preserving the bold markers instead of stripping them.

### Why bold wrapping is wrong

```
┌─────────────────────────────────────────────────────┐
│  Markdown headers are inherently bold by spec.      │
│  Every renderer (GitHub, VS Code, CommonMark)       │
│  renders `# Title` as bold H1 text.                 │
│                                                     │
│  Adding `**...**` is redundant and:                 │
│  1. Adds visual noise in raw markdown               │
│  2. May cause double-bold rendering in some parsers │
│  3. Deviates from gold standard                     │
└─────────────────────────────────────────────────────┘
```

### Fix approach

- **render_header()**: Remove bold wrapping; output `{prefix} {text}`
- **convert_standalone_bold_to_headers()**: Strip bold markers; output `## {text}`
- **Update tests**: 3 tests expected old `**...**` format → updated to expect clean headers
- **Setext headers**: Also remove bold wrapping for consistency
