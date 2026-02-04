# OODA-01: Orient

## Mission Re-Read ✅

**File**: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
**Goal**: F1 >= 0.95 by implementing pymupdf4llm algorithms with **pdfium-render** backend (pure Rust)

---

## Critical Constraints (from user)

1. **PURE RUST ONLY** - No Python, no subprocess, no FFI to Python
2. **PERMISSIVE LICENSE ONLY** - MIT or Apache-2.0 (no AGPL, no GPL)

---

## Analysis of Observations

### PDF Library Options Evaluated

| Library       | License           | Text Positions    | Status                 |
| ------------- | ----------------- | ----------------- | ---------------------- |
| lopdf         | MIT               | Inaccurate        | Current (F1=0.685)     |
| mupdf-rs      | AGPL-3.0          | Accurate          | **REJECTED** (license) |
| pdfium-render | MIT OR Apache-2.0 | Accurate (PDFium) | **SELECTED** ✅        |
| pdf-extract   | MIT               | Limited           | Not investigated       |

### pdfium-render Capabilities: ✅ CONFIRMED

From docs.rs/pdfium-render:

```rust
use pdfium_render::prelude::*;

let pdfium = Pdfium::default();
let document = pdfium.load_pdf_from_file("file.pdf", None)?;

for page in document.pages().iter() {
    let text = page.text()?;
    for char in text.chars() {
        let bounds = char.bounds();      // PdfRect with x0, y0, x1, y1
        let origin = char.origin();      // PdfPoint
        let font_size = char.font_size();
        let text = char.text();          // The actual character
    }
}
```

---

## Strategic Options (Pure Rust Only)

### Option A: Keep lopdf, Improve Algorithms

**Approach**: Fix text positioning within lopdf-based extraction

**Pros:**

- No new dependencies
- No runtime library requirement

**Cons:**

- Root cause is lopdf inaccuracy, not algorithms
- Months of work to fix CTM computation
- F1 improvement unlikely without accurate positions

### Option B: pdfium-render Backend (SELECTED ✅)

**Approach**: Replace lopdf with pdfium-render for text extraction

**Pros:**

- Accurate text positions from Google PDFium (Chromium's PDF engine)
- MIT OR Apache-2.0 license (permissive)
- Character-level bounding boxes
- Font information available
- 595 GitHub stars, actively maintained

**Cons:**

- Requires libpdfium.dylib/so at runtime
- Need to download pre-built binaries
- Dynamic linking adds deployment complexity

### Option C: Custom PDF Parser

**Approach**: Build accurate PDF text extraction from scratch

**Pros:**

- Full control
- No external dependencies

**Cons:**

- Years of work
- Not practical

---

## First Principles Analysis

### The Core Problem

Our F1 is 0.685 because **text positions are wrong**, not because our algorithms are wrong.

**Evidence from observations:**

```
Current:   ' can' at incorrectly computed position (merged with wrong text)
Expected:  Accurate character-by-character positions for layout analysis
```

### The Minimal Fix

If we get accurate text positions from pdfium-render, our existing algorithms should work better. Let's test this hypothesis.

### Why PDFium is Trustworthy

1. **Production-proven**: Powers Chrome/Chromium's PDF viewer (billions of users)
2. **Google-maintained**: Active development, regular security patches
3. **BSD-style license**: Permissive, commercial-friendly
4. **Accurate rendering**: Text positions are correct for display

---

## Decision Framework

```
┌─────────────────────────────────────────────────────────────────┐
│                  Implementation Priority Matrix                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Impact ▲                                                        │
│         │                                                        │
│    HIGH │  [B: pdfium-render] ◀── SELECTED                       │
│         │                                                        │
│   MEDIUM│  [A: lopdf fix]                                        │
│         │                                                        │
│    LOW  │  [C: custom parser]                                    │
│         │                                                        │
│         └────────────────────────────────────────────────────────▶
│              LOW          MEDIUM         HIGH       Effort       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Runtime Dependency Strategy

pdfium-render requires libpdfium at runtime. Strategy:

1. **Development**: Download from https://github.com/bblanchon/pdfium-binaries/releases
2. **CI/CD**: Include download step in build pipeline
3. **Production**: Bundle with application or document requirement
4. **Fallback**: Keep lopdf as fallback for environments without libpdfium

---

## Gap Analysis

| Current State        | Target State             | Gap                       |
| -------------------- | ------------------------ | ------------------------- |
| lopdf extraction     | pdfium-render extraction | Add dependency            |
| RawElement struct    | RawChar struct           | Add char-level extraction |
| Inaccurate positions | Accurate positions       | Use PdfPageTextChar       |
| F1 = 0.685           | F1 >= 0.95               | Algorithm tuning          |

---

## Recommended Approach

**Option B: pdfium-render Backend** is the clear winner:

1. Addresses root cause (inaccurate positions)
2. Pure Rust API (satisfies constraint #1)
3. MIT OR Apache-2.0 license (satisfies constraint #2)
4. Minimal implementation effort
5. Fast feedback on F1 improvement

---

## Next Step: Decide

Proceed to `decide.md` with specific implementation plan for pdfium-render integration.
