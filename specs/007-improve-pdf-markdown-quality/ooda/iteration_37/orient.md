# OODA IT37 — Orient

## Root Cause Analysis

### Garbled Text: First Principle
Natural language has a predictable word length distribution (mean ~5 chars, max ~30 chars).
PDF diagram text produces character sequences that violate this fundamental property:
- Characters positioned for visual elements overlap or abut
- No natural word boundaries (spaces) are present
- Result: 100+ char continuous strings

```
Normal text:   "Beekeepers play a crucial role..."  (avg word: 5 chars)
Garbled text:  "pbtBeekeepersinccrucialrolein..."  (one "word": 150 chars)
                                                     ↑ violates word length distribution
```

### Existing GarbledTextFilter Gaps
Current `is_garbled()` checks:
1. Short-word ratio (>35% unusual short words)
2. Isolated letters (>=4)
3. OCR fragment patterns

MISSING checks:
- Long-word detection (>40 chars = impossible in natural language)
- Low-space-ratio detection (<5% spaces in >80 char text)

### Header Spacing: First Principle
In natural language, section numbers are ALWAYS separated from titles:
- "1. Introduction" (period + space)
- "1 Introduction" (space)
- "§1 Introduction" (symbol + space)

NEVER "1Introduction" (digit directly touching uppercase). This is exclusively
a PDF extraction artifact from spans with sub-threshold gaps.

### Renderer Bypass
```
Pipeline: HeaderDetection → updates block.text → "1 INTRODUCTION"
Renderer: render_header() → reads block.spans → "1" + "INTRODUCTION" → "1INTRODUCTION"
                                                 ↑ spans NOT updated, block.text ignored
```
Fix: When block.text differs from span concatenation, use block.text.

## Impact Assessment
- Garbled text removal: ~1500 bytes of noise removed from page 3
- Header spacing: 4 major section headers corrected
- Both changes improve readability significantly with minimal regression risk
