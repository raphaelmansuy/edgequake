# OODA-09 Observe: Italic/Bold Font Detection Gap

## Data Gathered

### Current Metrics (Before)

- Quality: 0.724 (target: 0.95, gap: 0.226)
- Format: 0.470 (weakest dimension)
- v2_2512 Format: 0.299 (worst performing file)
- v2_2512 Italic: 0.000 (no italic detection at all!)

### Font Analysis (v2_2512.25072v1.pdf)

Using pymupdf to inspect actual fonts in the PDF:

```
NimbusRomNo9L-Medi: bold=True, italic=False (abstract text)
NimbusRomNo9L-MediItal: bold=True, italic=True (keywords like "Abstract")
NimbusRomNo9L-Regu: bold=False, italic=False (body text)
NimbusRomNo9L-ReguItal: bold=False, italic=True (emphasized text)
CMMI10: bold=False, italic=True (math italic)
CMSY6/8: bold=False, italic=True (math symbols ∗†)
```

### Font Detection Code (Before)

Two separate implementations:

1. `font_handling.rs` (lopdf backend)
2. `pymupdf_structs.rs` (pdfium backend)

Both had the same gap:

```rust
// Only detected "italic" and "oblique"
lower.contains("italic") || lower.contains("oblique")
```

This MISSED:

- `NimbusRomNo9L-ReguItal` → "Ital" abbreviation
- `NimbusRomNo9L-MediItal` → "Ital" abbreviation

### Bold Detection

Also discovered that "medi" was intentionally disabled:

```rust
// NOTE: We intentionally DON'T include "medi" because...
```

But `NimbusRomNo9L-Medi` is used for:

- Paper titles
- Abstract text (all bold in gold!)
- Figure captions

The rationale was wrong - bold fonts should be rendered bold.
