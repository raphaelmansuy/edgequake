# OODA IT37 — Observe

## Current Quality State

After IT36 (bullet list detection + content-hash images), converted the LightRAG academic paper:

- 16 pages, 239 blocks, 60421 bytes
- 27/27 bullet items properly formatted as `- **bold text**...`
- Headers: "#### 1INTRODUCTION" — number merged with title, no space
- Page 3: ~5 large garbled text blocks from Figure 1 diagram text extraction

## Issues Identified

### 1. Garbled Figure Diagram Text (Page 3)

```
pbtBeekeepersinccrucialroleinother...hryodeoloBEEKEEPER...
```

- 100+ character "words" with no spaces
- Multiple blocks of garbled concatenated text from vector diagram
- ~1500 bytes of noise on page 3

### 2. Header Number-Title Spacing

```
#### 1INTRODUCTION      → should be "#### 1 INTRODUCTION"
#### 2RETRIEVAL-AUGMENTED → should be "#### 2 RETRIEVAL-AUGMENTED"
#### 3THE LIGHTRAG       → should be "#### 3 THE LIGHTRAG"
```

- Section number directly adjacent to title text
- Caused by spans gap being below 15% font-size space threshold

### 3. render_header() Uses Spans Not block.text

- HeaderDetectionProcessor updates `block.text` but renderer ignores it
- `render_header()` calls `render_spans_styled()` which concatenates span texts
- Even if block.text is corrected, rendering still uses original span text

## Test Results

- 449 lib tests passing, 0 clippy warnings
- Elitizon: 84 blocks, 5332 bytes (no issues)
