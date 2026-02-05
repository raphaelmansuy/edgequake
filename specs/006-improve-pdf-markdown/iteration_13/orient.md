# OODA-13: Orient - Document Constants in pdfium.rs

## Analysis

### fs * 0.25 (space width)

**Purpose**: Synthesize space character width
- PDFium doesn't provide tight bounds for space characters
- We need to estimate the space width for word boundary detection
- 0.25 (25%) of font size is conservative

**Typography Background**:
- Proportional fonts: space width varies (0.2-0.3 of em)
- Monospace fonts: space width = character width (~0.6 of em)
- 0.25 is a good middle ground that works for both

**Alternative Considered**:
- Could use 0.33 (1/3 em) but that's too wide for dense text
- Could use 0.2 but that's too narrow for wide fonts

## Prioritization

This is a small change - just add WHY comment for the 0.25 factor.
