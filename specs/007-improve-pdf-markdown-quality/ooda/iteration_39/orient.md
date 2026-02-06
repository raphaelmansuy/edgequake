# OODA Iteration 39 — Orient

## Architecture Problem: Dual Classification Conflict

### The Classification Pipeline

```
PdfiumBackend::classify_blocks() → levels 1-4 based on font ratio
    ↓ (blocks already have level set)
StyleDetectionProcessor → SKIPS (block.level.is_some())
HeaderDetectionProcessor → SKIPS (block.level.is_some())
SectionPatternProcessor → processes SectionHeader blocks with level
```

### First-Principle Analysis

The pdfium backend's `classify_blocks` function uses a **single dimension** (font size ratio) for header classification. This is fundamentally insufficient because:

1. **Font size alone cannot distinguish headers from emphasized text** — figures, table captions, and inline callouts often use slightly larger fonts (1.2-1.3x body) without being headers.

2. **The threshold is too low** — 1.2x is a common font size for emphasized body text. The downstream `HeadingClassifier` correctly uses 1.4x as the minimum for H2.

3. **Levels 3 and 4 are unused by the renderer** — the rendering system was designed for the downstream processors that only generate levels 1-2. Levels 3-4 from the backend create unexpected heading depths.

### Impact

- **False `###` headers (level 3)**: Text fragments with font ratio 1.3-1.5x get classified as H3. These are body text, captions, or column-split artifacts.
- **Over-deep real headers (level 4 = `####`)**: Real section headers ("1. INTRODUCTION") have font ratio 1.2-1.3x and get classified as H4 instead of H2. The downstream SectionPatternProcessor would correctly assign H2 but never gets a chance because level is already set.

### Why the Backend Classification Exists

The pdfium backend classification was added as an early optimization: extract structural information during PDF parsing. However, the downstream processors are more sophisticated and handle the same task better. The backend classification is now a hindrance, not a help.

### Design Principle

**Classification responsibility should be single-owner.** Either the backend OR the processor chain should own header classification, not both. Since the processor chain has more signals (content analysis, pattern matching, prose detection), it should own this responsibility.
