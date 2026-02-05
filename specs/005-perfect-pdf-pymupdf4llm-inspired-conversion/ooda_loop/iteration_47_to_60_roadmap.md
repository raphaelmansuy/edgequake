# OODA Iterations 47-60: Quality Improvement Roadmap

## Overview

This document outlines the planned OODA iterations 47-60, focused on improving
the PDF-to-Markdown conversion quality metrics.

## Current Baseline (from OODA-44)

| Metric      | Score | Target |
| ----------- | ----- | ------ |
| Structure   | 0.417 | 0.90   |
| Format      | 0.659 | 0.95   |
| **Overall** | 0.786 | 0.95   |

---

## Phase 1: Code Quality (47-50)

### OODA-47: Reading Order Module Enhancement

- **Focus:** Improve `reading_order.rs` documentation
- **Goal:** Add ASCII diagrams explaining pymupdf4llm's smart sort algorithm
- **Impact:** Better maintainability, no quality change expected

### OODA-48: Geometric Clustering Documentation

- **Focus:** Document `geometric.rs` DBSCAN implementation
- **Goal:** Explain epsilon calculation and cluster merging
- **Impact:** Better maintainability

### OODA-49: pymupdf_renderer.rs Refactoring

- **Focus:** Split markdown rendering logic
- **Goal:** Separate bold/italic detection from block rendering
- **Impact:** Easier to fix format issues

### OODA-50: Test Infrastructure Cleanup

- **Focus:** Remove deprecated test files
- **Goal:** Clean test suite, single source of truth
- **Impact:** Faster CI, clearer test coverage

---

## Phase 2: Structure Improvements (51-55)

### OODA-51: Header Detection Refinement

- **Focus:** Improve font-size-based header detection
- **Goal:** Structure score 0.417 → 0.50
- **Changes:** Calibrate header_ratio threshold

### OODA-52: List Detection Enhancement

- **Focus:** Better indentation detection for nested lists
- **Goal:** Structure score 0.50 → 0.55
- **Changes:** Track indentation levels

### OODA-53: Section Number Preservation

- **Focus:** Keep section numbers (1., 2.1., etc.) in text
- **Goal:** Structure score 0.55 → 0.60
- **Changes:** Don't strip numeric prefixes

### OODA-54: Table Structure Detection

- **Focus:** Identify table boundaries
- **Goal:** Structure score 0.60 → 0.65
- **Changes:** Use grid alignment heuristics

### OODA-55: Code Block Detection

- **Focus:** Improve monospace font detection
- **Goal:** Structure score 0.65 → 0.70
- **Changes:** Better font family matching

---

## Phase 3: Format Improvements (56-60)

### OODA-56: Bold Span Accuracy

- **Focus:** Use PDFium font flags for bold detection
- **Goal:** Format score 0.659 → 0.70
- **Changes:** Use font_is_bold from RawChar

### OODA-57: Italic Span Accuracy

- **Focus:** Use PDFium font flags for italic detection
- **Goal:** Format score 0.70 → 0.75
- **Changes:** Use font_is_italic from RawChar

### OODA-58: Bullet Normalization

- **Focus:** Normalize Unicode bullets to standard markdown
- **Goal:** Format score 0.75 → 0.80
- **Changes:** Map all bullet chars to `-` or `*`

### OODA-59: Whitespace Handling

- **Focus:** Proper spacing between words and sentences
- **Goal:** Format score 0.80 → 0.85
- **Changes:** Detect natural word breaks

### OODA-60: Final Quality Validation

- **Focus:** Comprehensive quality test suite
- **Goal:** Verify all improvements
- **Expected:** Quality ~0.85

---

## Success Criteria

After OODA-60:

- Structure: ≥ 0.70 (from 0.417)
- Format: ≥ 0.85 (from 0.659)
- Overall: ≥ 0.85 (from 0.786)
- All 449+ tests passing
- Zero clippy warnings
