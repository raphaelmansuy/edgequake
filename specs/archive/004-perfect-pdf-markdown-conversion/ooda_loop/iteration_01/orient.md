# OODA Iteration 01 - Orient

## Root Cause Analysis: Qwen.pdf 0-byte Output

### Problem Statement

Qwen.pdf (852KB, 1 page) extracted to 0 bytes despite containing ~600 words of valid text.

### Investigation Path

#### 1. Font Analysis

- PDF uses **25 fonts**: 2 Type0 (CID fonts) + 23 Type3 (glyph procedure fonts)
- All Type3 fonts have valid `/Differences` arrays and `/ToUnicode` streams
- Python PDFMiner successfully extracted 689 characters (confirms PDF is valid)

#### 2. Content Stream Analysis

- Content has 595 `Tj` operators (text show operations)
- Format: `<00CD> Tj` (hex string format)
- CTM transform: `.23999999 0 0 -.23999999 0 792 cm`
  - Scale factor: 0.24 (scales up coordinates)
  - Y-flip: negative Y scale (flips coordinate system)
  - Translation: `0, 792` (moves origin to top-left)

#### 3. Y Coordinate Distribution

After CTM transform, text elements have Y coordinates:

- Minimum: 265.6 (icon font element)
- Maximum: 2452.5 (main text content)
- Page height: 792 points

#### 4. The Bug: OCR Layer Detection

The extraction engine has a heuristic to detect and filter OCR layers:

```rust
// OLD CODE (extraction_engine.rs line ~280)
let has_ocr_layer = actual_max_y > page_height * 2.5;
```

**Calculation for Qwen.pdf:**

- `actual_max_y = 2452.5`
- `page_height * 2.5 = 792 * 2.5 = 1980`
- `2452.5 > 1980` → TRUE → **OCR layer incorrectly detected**

#### 5. Filter Effect

When OCR layer is detected, filter bounds are set:

- `y_min = -0.5 * page_height = -396`
- `y_max = 2.0 * page_height = 1584`

All text with Y > 1584 was filtered out, leaving only 1 element (the icon font at Y=265.6).

### Root Cause Summary

| Factor            | Description                                                   |
| ----------------- | ------------------------------------------------------------- |
| **Primary Cause** | Absolute Y threshold heuristic fails for CTM-transformed PDFs |
| **Trigger**       | Type3 fonts with CTM transform scale up Y coordinates 4x      |
| **Symptom**       | 594 of 595 text elements filtered as "OCR layer"              |
| **Impact**        | Complete extraction failure (0 bytes output)                  |

### Why This Matters

- Type3 fonts are common in web-captured PDFs (like Qwen.pdf from qwen.ai)
- CTM transforms are standard PDF practice for coordinate normalization
- The absolute threshold approach cannot handle transformed coordinate systems
