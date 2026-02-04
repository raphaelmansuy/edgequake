# OODA-01: Decide

## Decision Statement

**DECISION**: Implement pdfium-render Backend (Pure Rust)

### Rationale

1. **Root Cause Targeting**: Our F1 is 0.685 primarily due to text position inaccuracy, not algorithm bugs
2. **Pure Rust**: pdfium-render provides a pure Rust API (satisfies user constraint)
3. **Permissive License**: MIT OR Apache-2.0 (satisfies user constraint, no AGPL)
4. **Production Proven**: PDFium powers Chrome's PDF viewer with billions of users
5. **Fast Feedback**: We can measure F1 improvement immediately after integration

---

## Implementation Plan for OODA-01

### Step 1: Download libpdfium for macOS

**Command**:

```bash
cd edgequake/crates/edgequake-pdf
mkdir -p lib
curl -L -o pdfium-mac-arm64.tgz \
  "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-mac-arm64.tgz"
tar -xzf pdfium-mac-arm64.tgz -C lib
rm pdfium-mac-arm64.tgz
```

**Result**: `lib/lib/libpdfium.dylib`

### Step 2: Add pdfium-render to Cargo.toml

**File**: `edgequake/crates/edgequake-pdf/Cargo.toml`

**Add**:

```toml
[dependencies]
pdfium-render = "0.8"
```

### Step 3: Create RawChar Structure

**File**: `edgequake/crates/edgequake-pdf/src/backend/mod.rs` (new)

```rust
/// A single character with exact position information from PDFium
#[derive(Debug, Clone)]
pub struct RawChar {
    pub char: char,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub font_size: f32,
    pub font_name: Option<String>,
    pub page_num: usize,
}

/// PDF extraction backend trait
pub trait PdfBackend {
    fn page_count(&self) -> usize;
    fn page_size(&self, page_num: usize) -> Option<(f32, f32)>;
    fn extract_page_chars(&self, page_num: usize) -> Result<Vec<RawChar>, PdfError>;
}
```

### Step 4: Implement PdfiumBackend

**File**: `edgequake/crates/edgequake-pdf/src/backend/pdfium.rs` (new)

```rust
use pdfium_render::prelude::*;
use super::{PdfBackend, RawChar};

pub struct PdfiumBackend<'a> {
    pdfium: &'a Pdfium,
    document: PdfDocument<'a>,
}

impl<'a> PdfiumBackend<'a> {
    pub fn new(pdfium: &'a Pdfium, path: &str) -> Result<Self, PdfError> {
        let document = pdfium.load_pdf_from_file(path, None)?;
        Ok(Self { pdfium, document })
    }
}

impl PdfBackend for PdfiumBackend<'_> {
    fn page_count(&self) -> usize {
        self.document.pages().len()
    }

    fn page_size(&self, page_num: usize) -> Option<(f32, f32)> {
        self.document.pages().get(page_num).ok().map(|page| {
            (page.width().value, page.height().value)
        })
    }

    fn extract_page_chars(&self, page_num: usize) -> Result<Vec<RawChar>, PdfError> {
        let page = self.document.pages().get(page_num)?;
        let text = page.text()?;

        let mut chars = Vec::new();
        for char_obj in text.chars() {
            if let Some(bounds) = char_obj.bounds() {
                chars.push(RawChar {
                    char: char_obj.text().chars().next().unwrap_or(' '),
                    x0: bounds.left.value,
                    y0: bounds.bottom.value,
                    x1: bounds.right.value,
                    y1: bounds.top.value,
                    font_size: char_obj.font_size().value,
                    font_name: None, // TODO: Extract font name
                    page_num,
                });
            }
        }

        Ok(chars)
    }
}
```

### Step 5: Test on Worst Document

**File**: `edgequake/crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.pdf`

**Actions**:

1. Create a simple test that loads this PDF with PdfiumBackend
2. Extract characters from first page
3. Print first 20 characters with positions
4. Verify positions look reasonable

### Step 6: Measure Quick F1

**Method**:

1. Create simple pipeline: pdfium chars -> group by line -> join to text -> compare to gold
2. Run on all 7 gold standard documents
3. Measure F1 improvement

---

## Resource Allocation

| Task                              | Time Estimate | Priority |
| --------------------------------- | ------------- | -------- |
| Download libpdfium                | 2 min         | P0       |
| Add pdfium-render dependency      | 1 min         | P0       |
| Create RawChar + PdfBackend trait | 10 min        | P0       |
| Implement PdfiumBackend           | 20 min        | P0       |
| Test on worst document            | 10 min        | P0       |
| Quick F1 measurement              | 15 min        | P1       |

**Total**: ~1 hour

---

## Success Criteria for OODA-01

OODA-01 is complete when:

1. [ ] libpdfium.dylib downloaded and available
2. [ ] pdfium-render added to Cargo.toml
3. [ ] `RawChar` and `PdfBackend` trait defined
4. [ ] `PdfiumBackend` implemented
5. [ ] Basic test passes with character extraction
6. [ ] Position accuracy verified visually

---

## Risk Mitigation

| Risk                             | Mitigation                                |
| -------------------------------- | ----------------------------------------- |
| libpdfium not found at runtime   | Set PDFIUM_DYNAMIC_LIB_PATH env var       |
| API differences in pdfium-render | Read docs.rs carefully, use prelude       |
| Build failures                   | Check pdfium-render version compatibility |

---

## Commit Checkpoint

After OODA-01, commit with message:

```
feat(pdf): OODA-01 - Add pdfium-render backend for accurate text extraction

- Add pdfium-render v0.8 dependency
- Create RawChar struct for character-level extraction
- Implement PdfiumBackend with PdfBackend trait
- Download libpdfium for macOS (arm64)
```

---

## Next Step: Act

Proceed to `act.md` to execute the implementation plan.
