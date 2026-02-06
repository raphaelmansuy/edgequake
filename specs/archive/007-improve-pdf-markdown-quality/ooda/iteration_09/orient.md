# Iteration 09: ORIENT - Analysis of Font Detection Gap

## Gap Analysis

### Current State

`looks_like_code()` detects only 4 font patterns:

1. `mono` - Catches most monospace fonts
2. `courier` - Classic PDF font
3. `consolas` - Windows code font
4. `source code` - Adobe font

### Missing Fonts Impact

| Font Family    | Usage Context                | Detection    |
| -------------- | ---------------------------- | ------------ |
| JetBrains Mono | Tech docs, programming blogs | ❌ (partial) |
| Monaco         | macOS terminal screenshots   | ❌           |
| Menlo          | macOS code editors           | ❌           |
| Fira Code      | VS Code default, tech PDFs   | ❌           |
| Inconsolata    | Academic papers, Google Docs | ❌           |
| Lucida Console | Windows terminal             | ❌           |
| Letter Gothic  | Classic typewriter           | ❌           |

### Root Cause

Incremental development - fonts were added as encountered.
No systematic audit against comprehensive font lists.

## Recommendation

Expand `looks_like_code()` with 20+ additional font patterns from:

1. Wikipedia "List of monospaced typefaces"
2. Common programming font surveys
3. System default fonts (macOS, Windows, Linux)

## Impact Assessment

- **Inline code**: More `\`backtick\`` wrapping in rendered markdown
- **Code blocks**: Better detection of code listings
- **False positives**: Low risk (monospace fonts are distinctive)

## Implementation Strategy

Add font patterns in categories:

1. Programming fonts (JetBrains, Fira, Hack, Iosevka)
2. System fonts (Menlo, Monaco, SF Mono, Lucida Console)
3. Classic fonts (Letter Gothic, Prestige, OCR)
