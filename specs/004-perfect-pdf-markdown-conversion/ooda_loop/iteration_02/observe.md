# OODA Iteration 02 - Observe

## Observation: Reading Order Incorrect in Qwen.pdf

### Date: 2024-02-02

### Problem Statement
After the OCR layer detection fix (OODA-01), Qwen.pdf now extracts content but in **wrong reading order**:
- "Beyond its Limits" appeared BEFORE "Pushing Qwen3-Max-Th"
- Visually, "Pushing" is the first line of the title, "Beyond" is the second

### Investigation Results

#### 1. Raw Y Coordinates (trace_content)
```
[1-20]  "Pushing Qwen3-Max-Th"  at Y=2329.1 (font_size=60)
[21-37] "Beyond its Limits"      at Y=2257.1 (font_size=60)
```

Higher Y (2329.1) should be TOP of page visually for this PDF.

#### 2. Normalized Coordinates (after extraction engine)
```
Block 8: 'Beyond its Limits'      bbox Y=557-617  → appears first (lower normalized Y)
Block 9: 'Pushing Qwen3-Max-Th'   bbox Y=629-689  → appears second
```

#### 3. CTM Transform Analysis
The PDF uses CTM: `.23999999 0 0 -.23999999 0 792 cm`
- d = -0.23999999 (NEGATIVE = Y axis flip)
- For negative d: higher original Y = higher on page visually

#### 4. Root Cause
The normalization code was detecting flipped coordinates AFTER OCR layer filtering:
- Original Y range: 265.6 to 2452.5 (span = 2186.9) → would detect flip
- After filtering: 1700.2 to 2452.5 (span = 752.2) → NOT detected as flipped

Since flip detection happened after filtering, the span was too small to trigger detection.

### Key Metrics
- Original Y span: 2186.9 points (2.76x page height) → SHOULD be flipped
- Filtered Y span: 752.2 points (0.95x page height) → NOT detected as flipped
- Detection threshold: 1.5x page height = 1188 points
