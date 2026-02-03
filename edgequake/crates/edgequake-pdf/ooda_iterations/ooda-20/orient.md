# OODA-20: Footnote Marker Cleanup - ORIENT

## Problem Classification
- Type: Text Cleanup
- Scope: Minor - single character at text start
- Priority: Low (minimal quality impact)

## First Principles Analysis

### What are footnote markers?
Academic papers use superscript symbols to reference footnotes:
- ⋆, *, †, ‡ etc. appear at the footnote text location
- In PDF, these are actual text glyphs, not annotations

### Why does this matter?
- Clean markdown should not have orphaned symbols
- Gold files created by humans strip these symbols
- Affects semantic matching even if score impact is small

## Solution Strategy
Add `strip_footnote_markers()` function to TextCleanupProcessor:
1. Identify footnote marker characters at text start
2. Strip the marker and following whitespace
3. Apply to block text and span text

## Risk Assessment
- Risk: Stripping legitimate leading symbols (e.g., list markers)
- Mitigation: Only strip recognized footnote symbols, not asterisks used for lists
