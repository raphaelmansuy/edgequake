# Iteration 01: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Analysis of Findings

### First Principles Assessment

**Core Question**: What is the minimum viable path to accurate PDF→Markdown with correct font styles?

**Answer**:

1. PDFium provides authoritative font style information via its API
2. The `pymupdf_grouper.rs` + `pymupdf_renderer.rs` already implement correct style propagation
3. ExtractionEngine (lopdf) is a redundant code path that loses style information

### Gap Analysis

| Aspect            | Current State                     | Target State        | Gap                  |
| ----------------- | --------------------------------- | ------------------- | -------------------- |
| Font style source | Dual (pdfium API + lopdf parsing) | Single (pdfium API) | Consolidate          |
| Style propagation | Pdfium works, lopdf loses styles  | All paths work      | Remove lopdf path    |
| Clippy warnings   | 14 warnings                       | 0 warnings          | Fix all              |
| WHY comments      | Sparse                            | Comprehensive       | Add explanations     |
| ASCII diagrams    | Few in code                       | Rich diagrams       | Add to complex algos |

### Risk Assessment

#### Option A: Remove lopdf backend completely

**Benefits**:

- Eliminates ~1347 lines of deprecated code
- Removes DRY violation (duplicate font detection)
- Simplifies maintenance
- Forces migration to correct (pdfium) path

**Risks**:

- Breaking change for code using `ExtractionEngine` directly
- May break tests that use lopdf feature
- Users without libpdfium binary cannot use the library

**Mitigation**:

- Keep `lopdf` feature flag but make it non-default
- Provide clear migration guide
- Ensure mock backend exists for testing

#### Option B: Keep lopdf, fix style propagation

**Benefits**:

- No breaking changes
- Pure Rust fallback remains

**Risks**:

- Perpetuates maintenance burden
- Font detection will never match pdfium quality
- DRY violations continue

**Recommendation**: **Option A** - Remove lopdf backend as default

### Root Cause of Style Loss (lopdf pipeline)

The lopdf pipeline loses styles because:

1. `extraction_engine.rs` creates `TextElement { is_bold, is_italic, ... }`
2. `text_grouping.rs` converts to `TextLine` → `Block`
3. `schema::Block` does **not** carry per-span style information
4. `renderers/markdown.rs` has no access to original style data

The pdfium pipeline avoids this by:

1. Using `RawChar { is_bold, is_italic }` from pdfium API
2. Converting to `pymupdf_structs::Span { flags }` which preserves styles
3. `pymupdf_renderer.rs` reads flags and applies markdown formatting

### SRP Analysis

| Module                 | Current Responsibilities                                                       | Violations                      |
| ---------------------- | ------------------------------------------------------------------------------ | ------------------------------- |
| `extraction_engine.rs` | Font parsing, content parsing, text grouping, block building, column detection | 5+ responsibilities             |
| `pdfium_backend.rs`    | PDFium orchestration, text grouping, block classification, schema conversion   | 4 responsibilities              |
| `font_handling.rs`     | Font info extraction, encoding resolution, bold/italic detection               | 3 responsibilities (acceptable) |

**Recommendation**: `extraction_engine.rs` should delegate to helper modules (which it already does partially).

### DRY Analysis

| Logic                      | Location 1                  | Location 2                 | Action                 |
| -------------------------- | --------------------------- | -------------------------- | ---------------------- |
| Bold detection from weight | `pdfium.rs:167-177`         | `font_handling.rs:175-190` | Single source of truth |
| Italic detection           | `pdfium.rs:164`             | `font_handling.rs:60-88`   | Single source of truth |
| Text grouping              | `layout/pymupdf_grouper.rs` | `backend/text_grouping.rs` | Consolidate            |

### Priority Matrix

| Task                           | Impact | Effort | Priority        |
| ------------------------------ | ------ | ------ | --------------- |
| Fix clippy warnings            | Medium | Low    | P1 (quick wins) |
| Add WHY comments to pdfium.rs  | High   | Low    | P1              |
| Add ASCII diagrams to grouper  | High   | Medium | P2              |
| Deprecate/remove lopdf default | High   | Medium | P2              |
| Consolidate text grouping      | Medium | High   | P3              |

### Technical Decisions

1. **Font style detection** → Use pdfium API exclusively (accurate)
2. **Text grouping** → Use `pymupdf_grouper.rs` exclusively (preserves styles)
3. **Markdown rendering** → Use `pymupdf_renderer.rs` exclusively (style-aware)
4. **lopdf feature** → Keep as non-default for edge cases (pure Rust env)

---

_Iteration 01 - Orient complete_
_Next: Decide - Prioritize specific changes_
