# OODA-05: Observe - Font Style Detection Gap

## Current State

**Quality**: 0.675 (target 0.95, gap 0.275)

### Score Breakdown

| Metric    | Score | Target | Gap   |
| --------- | ----- | ------ | ----- |
| ROUGE-L   | 0.702 | -      | -     |
| Word F1   | 0.899 | -      | -     |
| Structure | 0.350 | 1.0    | 0.650 |
| Format    | 0.343 | 1.0    | 0.657 |

### Key Observation

The **Structure** and **Format** scores are the main gaps. These relate to:

1. Header detection and level assignment
2. Bold/italic text marking

## Evidence Analysis

### 1. Font Style Detection

**Current approach** (font name parsing):

```rust
pub fn is_bold(&self) -> bool {
    self.font_name.as_ref().map(|n| {
        let lower = n.to_lowercase();
        lower.contains("bold") || lower.contains("black") || lower.contains("heavy")
    }).unwrap_or(false)
}
```

**Problem**: Font names like `NimbusRomNo9L-Medi` use "Medi" (Medium) instead of "Bold":

```
font=NimbusRomNo9L-Medi  ← Title (should be bold)
font=NimbusRomNo9L-Regu  ← Body (regular)
```

pymupdf4llm uses numeric **flags**:

```python
bold = s["flags"] & 16 or s["char_flags"] & 8
italic = s["flags"] & 2
```

These flags are from the PDF font descriptor, not the font name.

### 2. Header Level Assignment

**Current approach** (ratio-based):

```rust
let ratio = dominant_size / body_font_size;
let level = if ratio >= 2.0 { 1 }
            else if ratio >= 1.7 { 2 }
            else if ratio >= 1.5 { 3 }
            ...
```

**pymupdf4llm approach** (size-ranked):

```python
sizes = sorted([f for f in fontsizes.keys() if f > self.body_limit], reverse=True)[:6]
for i, size in enumerate(sizes, start=1):
    self.header_id[size] = "#" * i + " "
```

The key difference: pymupdf4llm assigns H1 to the **largest** font size found, H2 to the second largest, etc. Our ratio-based approach may not match the gold standard.

### 3. Example Mismatch

**Gold file** (v2_2512.25072v1.pymupdf.gold.md):

```markdown
## **Coordinated Humanoid Manipulation with Choice Policies**
```

**Our output**:

```markdown
### Coordinated Humanoid Manipulation with Choice Policies
```

Issues:

1. Header level: ## (H2) vs ### (H3)
2. Bold markers: `**...**` vs plain text

## Root Causes

1. **Font name patterns incomplete**: Missing "Medi", "Semi", "Demi" as bold indicators
2. **Header level calculation**: Not matching pymupdf4llm's size-ranked approach
3. **No font weight extraction**: PDFium exposes `FPDFText_GetFontWeight()` but we don't use it

## Next Steps

1. Improve font name pattern matching for bold detection
2. Consider implementing size-ranked header levels like pymupdf4llm
3. Future: Add font weight extraction from PDFium for more accurate bold detection

---

**Timestamp**: 2025-01-27
