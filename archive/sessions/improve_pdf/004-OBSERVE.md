# OODA Loop 4 - OBSERVE

## Focus: Missing Tables in agent_2510

### Discovery

**agent_2510.09244v1:**

- Gold: 22 table lines
- Generated: 0 table lines
- **Issue:** 100% table detection failure

### Root Cause Analysis

Inspected gold markdown table at line 306-311:

```markdown
| Modality       | Input Format | Tool Dependencies | Strengths    | Limitations |
| -------------- | ------------ | ----------------- | ------------ | ----------- |
| **Text-Based** | Plain text   | None              | Low overhead | Text-only   |
```

**First Principles Discovery:**

These tables in the original PDF are **whitespace-aligned text tables**, NOT lattice tables with graphical lines!

**Evidence:**

- Tables formatted with `|` delimiters in gold
- But original PDF has NO PdfLine elements forming a grid
- Tables use text alignment/spacing only (like ASCII art tables)

**Current Lattice Detector:**

```rust
fn extract_from_lattice(lines: &[PdfLine], text: &[TextElement])
```

**Problem:** Only looks for graphical lines (`PdfLine` elements). Whitespace-based tables have ZERO `PdfLine` elements, so they're completely invisible to the lattice detector.

### First Principles of Table Encoding

PDFs encode tables in TWO fundamentally different ways:

1. **Lattice Tables:** Explicit graphical lines (h_lines, v_lines) forming grid
   - Example: 2900_Goyal PDF (detected ✓)
2. **Whitespace Tables:** Text positioned in columns using spaces/tabs
   - Example: agent_2510 PDF (NOT detected ❌)
   - Detection requires: Column alignment analysis of text positions

### Impact

This explains the 2.4% table accuracy:

- agent_2510 contributes 0/22 tables (major penalty)
- Our lattice detector is fundamentally blind to whitespace tables
