# Implementation Priorities

> **OODA Loop - Act**: Prioritized roadmap with ROI analysis, dependencies, and phased execution plan.

**Planning Horizon**: 12 weeks (Q1 2026)  
**Team Size Assumption**: 1-2 engineers  
**Risk Level**: Medium (production codebase, active users)

---

## Executive Summary

### Effort/Impact Matrix

```
High Impact │ P0A: Parallel    │ P0C: OCR        │
    ▲       │ Processing       │ Integration     │
    │       │ (2w, 3.8x)       │ (4w, +40pts)    │
    │       ├──────────────────┼─────────────────┤
    │       │ P0B: Algorithm   │ P1B: Math       │
    │       │ Optimization     │ Formulas        │
    │       │ (2w, 1.6x)       │ (2w, +30pts)    │
    │       ├──────────────────┼─────────────────┤
    │       │ P1A: Error       │ P2A: Config     │
    │       │ Recovery         │ System          │
Low Impact  │ (3d, +20pts)     │ (1w, UX)        │
            └──────────────────┴─────────────────┘
             Low Effort          High Effort
```

### Priority Rankings

| Priority | Feature                | Effort  | Impact              | ROI        | Dependencies      |
| -------- | ---------------------- | ------- | ------------------- | ---------- | ----------------- |
| **P0A**  | Parallel Processing    | 2 weeks | 3.8x speedup        | 🔥🔥🔥🔥🔥 | None              |
| **P0B**  | Algorithm Optimization | 2 weeks | 1.6x speedup        | 🔥🔥🔥🔥   | None              |
| **P0C**  | OCR Integration        | 4 weeks | +40 quality pts     | 🔥🔥🔥     | P0A (parallelism) |
| **P1A**  | Error Recovery         | 3 days  | +20 reliability pts | 🔥🔥🔥     | None              |
| **P1B**  | Math Formulas          | 2 weeks | +30 quality pts     | 🔥🔥🔥     | P0B (font info)   |
| **P1C**  | Testing Expansion      | 6 weeks | +18% coverage       | 🔥🔥       | Ongoing           |
| **P2A**  | Plugin System          | 3 weeks | Extensibility       | 🔥🔥       | P0B (refactor)    |
| **P2B**  | Streaming API          | 2 weeks | UX improvement      | 🔥🔥       | P0A (parallelism) |
| **P2C**  | Config System          | 1 week  | Developer UX        | 🔥         | None              |

---

## Phase 1: Quick Wins (Week 1-2)

### Goal: Immediate performance and reliability improvements

**Timeline**: 2 weeks  
**Risk**: Low  
**Blockers**: None

---

### Task 1.1: Parallel Page Processing (P0A)

**Effort**: 5 days  
**Impact**: 3.8x speedup on 4-core machines  
**ROI**: ⭐⭐⭐⭐⭐

**Implementation Steps**:

```rust
// Day 1-2: Add rayon parallelism
// File: src/extractor.rs

use rayon::prelude::*;

impl PdfExtractor {
    pub fn extract_document_parallel(&self, pdf_bytes: &[u8]) -> Result<Document> {
        let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)?;
        let page_ids = self.backend.get_page_ids(&lopdf_doc)?;

        // Parallel page extraction
        let pages: Result<Vec<Page>> = page_ids
            .par_iter()
            .enumerate()
            .map(|(idx, page_id)| {
                self.backend.extract_page(&lopdf_doc, *page_id, idx)
            })
            .collect();

        Ok(Document { pages: pages?, ..Default::default() })
    }
}
```

**Validation**:

- [ ] Run benchmarks: `cargo bench extraction_scaling`
- [ ] Verify 3.5x+ speedup on 4-core machine
- [ ] Test thread safety with `cargo test --features parallel`
- [ ] Check CPU utilization (should be 90%+ on all cores)

**Commit Message**: `perf(pdf): Add parallel page extraction with rayon (3.8x speedup)`

---

### Task 1.2: Error Recovery (P1A)

**Effort**: 3 days  
**Impact**: +20 reliability points  
**ROI**: ⭐⭐⭐⭐

**Implementation Steps**:

```rust
// Day 1: Add ExtractionResult type
// File: src/extractor.rs

pub struct ExtractionResult {
    pub document: Document,
    pub errors: Vec<PageError>,
    pub warnings: Vec<String>,
}

// Day 2: Implement graceful degradation
impl PdfExtractor {
    pub fn extract_with_recovery(&self, pdf_bytes: &[u8]) -> Result<ExtractionResult> {
        let mut pages = Vec::new();
        let mut errors = Vec::new();

        for (idx, page_id) in page_ids.iter().enumerate() {
            match self.backend.extract_page(&lopdf_doc, *page_id, idx) {
                Ok(page) => pages.push(page),
                Err(e) if e.is_recoverable() => {
                    pages.push(Page::placeholder(idx));
                    errors.push(PageError { page: idx, error: e, recoverable: true });
                }
                Err(e) => {
                    errors.push(PageError { page: idx, error: e, recoverable: false });
                }
            }
        }

        Ok(ExtractionResult { document: Document { pages, .. }, errors, warnings: vec![] })
    }
}

// Day 3: Add is_recoverable() to PdfError
impl PdfError {
    pub fn is_recoverable(&self) -> bool {
        matches!(self,
            PdfError::MalformedStream { .. } |
            PdfError::UnsupportedFont { .. } |
            PdfError::DecodingFailed { .. }
        )
    }
}
```

**Validation**:

- [ ] Test with 10 corrupted PDFs
- [ ] Verify partial extraction works
- [ ] Validate error categorization

**Commit Message**: `fix(pdf): Add graceful error recovery for page-level failures`

---

### Task 1.3: Clippy Cleanup (P1C)

**Effort**: 1 day  
**Impact**: Code quality improvement  
**ROI**: ⭐⭐

```bash
# Run clippy auto-fix
cargo clippy --fix --lib -p edgequake-pdf --allow-dirty

# Verify no warnings
cargo clippy --package edgequake-pdf -- -D warnings
```

**Validation**:

- [ ] Zero clippy warnings
- [ ] All tests pass

**Commit Message**: `style(pdf): Fix all clippy warnings and improve code quality`

---

## Phase 2: Algorithm Optimization (Week 3-4)

### Goal: O(n²) → O(n log n) algorithmic improvements

**Timeline**: 2 weeks  
**Risk**: Medium (algorithmic changes)  
**Dependencies**: None

---

### Task 2.1: Union-Find Connected Components (P0B)

**Effort**: 5 days  
**Impact**: Table detection 10x faster  
**ROI**: ⭐⭐⭐⭐⭐

**Implementation Steps**:

```rust
// Day 1-2: Implement union-find data structure
// File: src/backend/union_find.rs

pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    pub fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    pub fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];  // Path compression
            x = self.parent[x];
        }
        x
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x == root_y { return; }

        // Union by rank
        if self.rank[root_x] < self.rank[root_y] {
            self.parent[root_x] = root_y;
        } else if self.rank[root_x] > self.rank[root_y] {
            self.parent[root_y] = root_x;
        } else {
            self.parent[root_y] = root_x;
            self.rank[root_x] += 1;
        }
    }
}

// Day 3-4: Replace connected_components() in lattice.rs
impl LatticeEngine {
    fn connected_components_fast(&self, grid: &Grid) -> Vec<Vec<(usize, usize)>> {
        let cell_count = grid.rows * grid.cols;
        let mut uf = UnionFind::new(cell_count);

        // Union adjacent cells
        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let idx = row * grid.cols + col;

                // Right neighbor
                if col + 1 < grid.cols {
                    uf.union(idx, idx + 1);
                }

                // Bottom neighbor
                if row + 1 < grid.rows {
                    uf.union(idx, idx + grid.cols);
                }
            }
        }

        // Group by root
        let mut components: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        for row in 0..grid.rows {
            for col in 0..grid.cols {
                let idx = row * grid.cols + col;
                let root = uf.find(idx);
                components.entry(root).or_default().push((row, col));
            }
        }

        components.into_values().collect()
    }
}
```

**Validation**:

- [ ] Benchmark: `cargo bench table_detection_complexity`
- [ ] Verify 10x+ speedup for 1000-cell tables
- [ ] Test correctness with existing test suite

**Commit Message**: `perf(pdf): Replace O(n²) connected components with union-find (10x faster)`

---

### Task 2.2: R-tree Spatial Indexing (P0B)

**Effort**: 5 days  
**Impact**: 5x faster nearest neighbor queries  
**ROI**: ⭐⭐⭐⭐

**Implementation Steps**:

```rust
// Day 1-2: Add rstar dependency
// Cargo.toml
[dependencies]
rstar = "0.11"

// Day 3-4: Implement R-tree for block deduplication
// File: src/backend/element_processing.rs

use rstar::{RTree, AABB};

impl ElementProcessor {
    fn deduplicate_with_rtree(&self, blocks: Vec<Block>) -> Vec<Block> {
        // Build R-tree of block bounding boxes
        let tree: RTree<BlockNode> = RTree::bulk_load(
            blocks.iter().enumerate().map(|(idx, block)| {
                BlockNode { idx, bbox: block.bbox.clone() }
            }).collect()
        );

        let mut keep = vec![true; blocks.len()];

        for (i, block) in blocks.iter().enumerate() {
            if !keep[i] { continue; }

            // Query nearby blocks (within 2 pixels)
            let search_box = block.bbox.expand(2.0);

            for neighbor in tree.locate_in_envelope(&search_box.to_aabb()) {
                let j = neighbor.idx;
                if i >= j || !keep[j] { continue; }

                let other = &blocks[j];
                if self.is_duplicate(block, other) {
                    keep[j] = false;
                }
            }
        }

        blocks.into_iter().enumerate()
            .filter_map(|(i, block)| if keep[i] { Some(block) } else { None })
            .collect()
    }
}

struct BlockNode {
    idx: usize,
    bbox: BoundingBox,
}

impl rstar::RTreeObject for BlockNode {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bbox.x1, self.bbox.y1],
            [self.bbox.x2, self.bbox.y2],
        )
    }
}
```

**Validation**:

- [ ] Benchmark: O(n log n) vs O(n²)
- [ ] Verify 5x+ speedup for 1000+ blocks
- [ ] Test correctness with golden files

**Commit Message**: `perf(pdf): Use R-tree for spatial indexing (5x faster deduplication)`

---

## Phase 3: Feature Expansion (Week 5-8)

### Goal: Add OCR and math formula support

**Timeline**: 4 weeks  
**Risk**: Medium-High (new features)  
**Dependencies**: Phase 1 (parallelism), Phase 2 (font info)

---

### Task 3.1: OCR Integration (P0C)

**Effort**: 4 weeks  
**Impact**: +40 quality points  
**ROI**: ⭐⭐⭐

**Week 1: Tesseract Integration**

```rust
// Add dependencies
// Cargo.toml
[dependencies]
tesseract = "0.14"
image = "0.24"

// Implement OCR pipeline
// File: src/ocr/mod.rs

use tesseract::Tesseract;

pub struct OcrEngine {
    tess: Tesseract,
}

impl OcrEngine {
    pub fn new(lang: &str) -> Result<Self> {
        let tess = Tesseract::new(None, Some(lang))?;
        Ok(Self { tess })
    }

    pub fn recognize_image(&mut self, image_bytes: &[u8]) -> Result<OcrResult> {
        self.tess.set_image_from_mem(image_bytes)?;

        let text = self.tess.get_text()?;
        let confidence = self.tess.get_mean_confidence();

        Ok(OcrResult { text, confidence })
    }
}
```

**Week 2: Page Rendering**

```rust
// Render PDF page to image
use pdf_render::Renderer;

impl PdfExtractor {
    fn render_page_to_image(&self, pdf: &[u8], page_num: usize, dpi: u32)
        -> Result<Vec<u8>> {

        let renderer = Renderer::new(pdf)?;
        let image = renderer.render_page(page_num, dpi)?;

        // Convert to PNG bytes
        let mut png_bytes = Vec::new();
        image.write_to(&mut png_bytes, ImageFormat::Png)?;

        Ok(png_bytes)
    }
}
```

**Week 3: Hybrid Page Detection**

```rust
// Detect scanned vs native text pages
impl PdfExtractor {
    fn detect_page_type(&self, page: &Page) -> PageType {
        let text_density = page.blocks.len() as f32 / page.bbox.area();

        if text_density < 0.001 && page.images.len() > 0 {
            PageType::Scanned
        } else if text_density > 0.01 {
            PageType::Native
        } else {
            PageType::Hybrid  // Mix of native + scanned
        }
    }
}
```

**Week 4: OCR Result Merging**

```rust
// Merge OCR with native text
impl PdfExtractor {
    pub async fn extract_with_ocr(&self, pdf: &[u8], config: OcrConfig)
        -> Result<Document> {

        let doc = self.extract_document(pdf)?;

        for (idx, page) in doc.pages.iter_mut().enumerate() {
            match self.detect_page_type(page) {
                PageType::Scanned => {
                    let image = self.render_page_to_image(pdf, idx, config.dpi)?;
                    let ocr_result = self.ocr_engine.recognize_image(&image)?;
                    page.blocks = self.convert_ocr_to_blocks(ocr_result);
                }
                PageType::Hybrid => {
                    // Merge native + OCR
                    let ocr_blocks = self.extract_ocr_for_images(page)?;
                    page.blocks.extend(ocr_blocks);
                }
                PageType::Native => {
                    // No OCR needed
                }
            }
        }

        Ok(doc)
    }
}
```

**Validation**:

- [ ] Test with 20 scanned documents
- [ ] Verify 85%+ OCR accuracy
- [ ] Benchmark OCR overhead (should be <2s/page)
- [ ] Test hybrid document handling

**Commit Message**: `feat(pdf): Add Tesseract OCR support for scanned documents`

---

### Task 3.2: Math Formula Detection (P1B)

**Effort**: 2 weeks  
**Impact**: +30 quality points  
**ROI**: ⭐⭐⭐

**Week 1: Symbol Detection**

```rust
// File: src/formula/detector.rs

pub struct FormulaDetector {
    symbol_map: HashMap<char, &'static str>,
}

impl FormulaDetector {
    pub fn detect_formulas(&self, page: &Page) -> Vec<Formula> {
        let mut formulas = Vec::new();

        for block in &page.blocks {
            let math_density = self.count_math_symbols(&block.text)
                / block.text.len() as f32;

            if math_density > 0.15 {
                if let Some(formula) = self.reconstruct_formula(block) {
                    formulas.push(formula);
                }
            }
        }

        formulas
    }

    fn count_math_symbols(&self, text: &str) -> usize {
        text.chars().filter(|c| self.symbol_map.contains_key(c)).count()
    }
}
```

**Week 2: LaTeX Reconstruction**

```rust
impl FormulaDetector {
    fn reconstruct_formula(&self, block: &Block) -> Option<Formula> {
        let mut latex = String::new();

        // Detect superscripts/subscripts from Y-offset
        for span in &block.spans {
            let y_offset = span.bbox.y1 - block.bbox.y1;

            if y_offset < -2.0 {
                latex.push_str(&format!("^{{{}}}", self.convert_symbols(&span.text)));
            } else if y_offset > 2.0 {
                latex.push_str(&format!("_{{{}}}", self.convert_symbols(&span.text)));
            } else {
                latex.push_str(&self.convert_symbols(&span.text));
            }
        }

        let confidence = self.calculate_confidence(&latex);

        Some(Formula {
            latex,
            bbox: block.bbox.clone(),
            confidence,
        })
    }

    fn convert_symbols(&self, text: &str) -> String {
        let mut result = String::new();
        for ch in text.chars() {
            if let Some(latex) = self.symbol_map.get(&ch) {
                result.push_str(latex);
            } else {
                result.push(ch);
            }
        }
        result
    }
}
```

**Validation**:

- [ ] Test with 100 arXiv papers
- [ ] Verify 90%+ formula accuracy
- [ ] Compare with gold standard LaTeX

**Commit Message**: `feat(pdf): Add math formula detection and LaTeX conversion`

---

## Phase 4: Testing & Quality (Ongoing, Week 1-12)

### Goal: 90%+ test coverage, continuous quality validation

**Timeline**: 6 weeks (parallel with other phases)  
**Risk**: Low  
**Dependencies**: Ongoing

---

### Task 4.1: Unit Test Expansion (P1C)

**Week 1-2**: Add 91 unit tests

- [ ] 20 font encoding tests
- [ ] 25 math formula tests
- [ ] 18 table detection tests
- [ ] 15 error handling tests
- [ ] 13 edge case tests

**Week 3-4**: Integration tests

- [ ] 10 arXiv paper tests
- [ ] 5 financial report tests
- [ ] 5 legal document tests
- [ ] 10 multilingual tests
- [ ] 10 large document tests

**Week 5-6**: Performance benchmarks

- [ ] 10 extraction scaling benchmarks
- [ ] 10 algorithm complexity benchmarks
- [ ] 10 memory usage benchmarks

**Validation**:

- [ ] Run coverage: `cargo tarpaulin --out Html`
- [ ] Verify 90%+ coverage
- [ ] All tests pass on CI

**Commit Message**: `test(pdf): Expand test suite to 400+ tests with 90% coverage`

---

### Task 4.2: Fuzzing Setup (P1C)

**Effort**: 3 days  
**Impact**: Bug discovery  
**ROI**: ⭐⭐⭐

```bash
# Day 1: Install cargo-fuzz
cargo install cargo-fuzz

# Day 2: Create fuzz target
cargo fuzz init
echo '
#![no_main]
use libfuzzer_sys::fuzz_target;
use edgequake_pdf::PdfExtractor;

fuzz_target!(|data: &[u8]| {
    let extractor = PdfExtractor::new();
    let _ = extractor.extract_document(data);
});
' > fuzz/fuzz_targets/pdf_parsing.rs

# Day 3: Run fuzzing
cargo fuzz run pdf_parsing -- -max_total_time=3600
```

**Commit Message**: `test(pdf): Add continuous fuzzing for PDF parsing`

---

## Phase 5: Advanced Features (Week 9-12)

### Goal: Plugin system, streaming API, configuration

**Timeline**: 4 weeks  
**Risk**: Medium  
**Dependencies**: Phase 2 (refactoring)

---

### Task 5.1: Plugin System (P2A)

**Effort**: 3 weeks  
**Impact**: Extensibility  
**ROI**: ⭐⭐

**Implementation**: See [ARCHITECTURE_EVOLUTION.md](ARCHITECTURE_EVOLUTION.md) Section 1

**Validation**:

- [ ] Test with 3rd party processor plugin
- [ ] Verify plugin loading works
- [ ] Benchmark plugin overhead (<5%)

**Commit Message**: `feat(pdf): Add ProcessorPlugin trait for extensibility`

---

### Task 5.2: Streaming API (P2B)

**Effort**: 2 weeks  
**Impact**: UX improvement  
**ROI**: ⭐⭐

**Implementation**: See [ARCHITECTURE_EVOLUTION.md](ARCHITECTURE_EVOLUTION.md) Section 2

**Validation**:

- [ ] Test with 200-page document
- [ ] Verify progress callbacks work
- [ ] Measure memory usage (should be constant)

**Commit Message**: `feat(pdf): Add streaming API with progress callbacks`

---

### Task 5.3: Configuration System (P2C)

**Effort**: 1 week  
**Impact**: Developer UX  
**ROI**: ⭐⭐

**Implementation**: See [ARCHITECTURE_EVOLUTION.md](ARCHITECTURE_EVOLUTION.md) Section 5

**Validation**:

- [ ] Test TOML config loading
- [ ] Verify config validation
- [ ] Test environment variable override

**Commit Message**: `feat(pdf): Add TOML-based configuration system`

---

## Dependency Graph

```
                    ┌───────────────┐
                    │  Phase 1      │
                    │  Quick Wins   │
                    │  (Week 1-2)   │
                    └───────┬───────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
              ▼             ▼             ▼
      ┌───────────┐ ┌───────────┐ ┌───────────┐
      │ Parallel  │ │  Error    │ │  Clippy   │
      │ Processing│ │  Recovery │ │  Cleanup  │
      │ (5d, P0A) │ │ (3d, P1A) │ │ (1d, P1C) │
      └─────┬─────┘ └───────────┘ └───────────┘
            │
            │ ┌───────────────┐
            │ │  Phase 2      │
            └▶│  Algorithms   │
              │  (Week 3-4)   │
              └───────┬───────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│ Union-Find│ │  R-tree   │ │  Hash Map │
│ (5d, P0B) │ │ (5d, P0B) │ │ (2d, P0B) │
└─────┬─────┘ └───────────┘ └───────────┘
      │
      │       ┌───────────────┐
      │       │  Phase 3      │
      └───────▶  Features     │
              │  (Week 5-8)   │
              └───────┬───────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│    OCR    │ │   Math    │ │   Table   │
│Integration│ │  Formulas │ │  Merged   │
│ (4w, P0C) │ │ (2w, P1B) │ │  Cells    │
└───────────┘ └───────────┘ │ (5d, P1)  │
                            └───────────┘

              ┌───────────────┐
              │  Phase 4      │
              │  Testing      │
              │  (Week 1-12)  │◀── Continuous
              └───────┬───────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│   Unit    │ │Integration│ │  Fuzzing  │
│   Tests   │ │   Tests   │ │   Setup   │
│(+91, P1C) │ │(+40, P1C) │ │ (3d, P1C) │
└───────────┘ └───────────┘ └───────────┘

              ┌───────────────┐
              │  Phase 5      │
              │  Advanced     │
              │  (Week 9-12)  │
              └───────┬───────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐
│  Plugin   │ │ Streaming │ │  Config   │
│  System   │ │    API    │ │  System   │
│ (3w, P2A) │ │ (2w, P2B) │ │ (1w, P2C) │
└───────────┘ └───────────┘ └───────────┘
```

---

## Risk Mitigation

### High-Risk Items

| Risk                                    | Probability | Impact | Mitigation                                                  |
| --------------------------------------- | ----------- | ------ | ----------------------------------------------------------- |
| **OCR accuracy insufficient**           | Medium      | High   | Implement confidence thresholds, fallback to manual review  |
| **Parallel extraction race conditions** | Low         | High   | Extensive testing, use thread-safe data structures          |
| **Performance regression**              | Medium      | Medium | Automated benchmarks in CI, alerting on >10% slowdown       |
| **Breaking API changes**                | Low         | High   | Deprecation warnings, migration guide, semantic versioning  |
| **Test maintenance burden**             | High        | Low    | Generate tests from templates, automate golden file updates |

---

## Success Metrics

### Phase 1 (Week 1-2)

- [ ] Parallel extraction: 3.5x+ speedup
- [ ] Error recovery: 90%+ of errors recoverable
- [ ] Clippy: Zero warnings

### Phase 2 (Week 3-4)

- [ ] Union-find: 10x faster for 1000-cell tables
- [ ] R-tree: 5x faster deduplication
- [ ] Hash map: 2x faster O(n²) → O(n)

### Phase 3 (Week 5-8)

- [ ] OCR: 85%+ accuracy on scanned docs
- [ ] Formulas: 90%+ LaTeX accuracy
- [ ] Tables: 95%+ merged cell detection

### Phase 4 (Week 1-12)

- [ ] Test coverage: 90%+
- [ ] Test count: 400+ tests
- [ ] CI: <10 minute build time

### Phase 5 (Week 9-12)

- [ ] Plugins: 3+ community processors
- [ ] Streaming: Constant memory for 200-page docs
- [ ] Config: 100% feature coverage

---

## Timeline Visualization

```
Week │ Phase                        │ Deliverables
─────┼──────────────────────────────┼─────────────────────────────────
  1  │ ████ Phase 1: Quick Wins     │ Parallel processing, error recovery
  2  │ ████                          │ Clippy cleanup
─────┼──────────────────────────────┼─────────────────────────────────
  3  │ ████ Phase 2: Algorithms     │ Union-find, R-tree
  4  │ ████                          │ Hash map dedup
─────┼──────────────────────────────┼─────────────────────────────────
  5  │ ████ Phase 3: OCR (Week 1)   │ Tesseract integration
  6  │ ████          OCR (Week 2)   │ Page rendering
  7  │ ████          OCR (Week 3)   │ Hybrid detection
  8  │ ████ Math Formulas (Week 1)  │ Symbol detection, LaTeX conversion
─────┼──────────────────────────────┼─────────────────────────────────
  9  │ ████ Phase 5: Plugin System  │ ProcessorPlugin trait
 10  │ ████          Plugin System  │ Dynamic loading
 11  │ ████ Streaming API (Week 1)  │ extract_pages_stream()
 12  │ ████ Config System           │ TOML config, validation
─────┼──────────────────────────────┼─────────────────────────────────
1-12 │ ▓▓▓▓ Phase 4: Testing        │ +161 tests, fuzzing, benchmarks
     │ (Continuous, parallel track) │ 90% coverage
```

---

## Next Actions (This Week)

### Monday-Tuesday: Parallel Processing (P0A, Day 1-2)

```bash
# Branch
git checkout -b feat/parallel-extraction

# Implement
vim edgequake/crates/edgequake-pdf/src/extractor.rs
# Add rayon parallelism as shown in Task 1.1

# Test
cargo test --package edgequake-pdf --lib

# Benchmark
cargo bench extraction_scaling

# Commit
git add edgequake/crates/edgequake-pdf/src/
git commit -m "perf(pdf): Add parallel page extraction with rayon (3.8x speedup)"
```

### Wednesday-Thursday: Finish Parallel + Error Recovery (P0A Day 3-5, P1A Day 1-2)

```bash
# Complete parallel implementation
# Start error recovery

vim edgequake/crates/edgequake-pdf/src/extractor.rs
# Add ExtractionResult, extract_with_recovery()

# Test
cargo test error_recovery --nocapture

# Commit
git add -A
git commit -m "fix(pdf): Add graceful error recovery for page-level failures"
```

### Friday: Clippy + Code Review (P1C, P0A/P1A review)

```bash
# Clippy cleanup
cargo clippy --fix --lib -p edgequake-pdf --allow-dirty

# Test
cargo test --package edgequake-pdf
cargo clippy --package edgequake-pdf -- -D warnings

# Commit
git add -A
git commit -m "style(pdf): Fix all clippy warnings and improve code quality"

# Create PR
git push origin feat/parallel-extraction
gh pr create --title "Phase 1: Parallel extraction + error recovery" \
             --body "3.8x speedup, +20 reliability points, zero clippy warnings"
```

---

## Appendix: ROI Calculations

### P0A: Parallel Processing

- **Effort**: 5 days × 1 engineer = 5 person-days
- **Impact**: 3.8x speedup = 380% performance gain
- **ROI**: 380% / 5 days = **76% gain per day**

### P0B: Algorithm Optimization

- **Effort**: 10 days × 1 engineer = 10 person-days
- **Impact**: 1.6x speedup = 160% performance gain
- **ROI**: 160% / 10 days = **16% gain per day**

### P0C: OCR Integration

- **Effort**: 20 days × 1 engineer = 20 person-days
- **Impact**: +40 quality points (enables scanned docs)
- **ROI**: 40 pts / 20 days = **2 pts per day**

### P1A: Error Recovery

- **Effort**: 3 days × 1 engineer = 3 person-days
- **Impact**: +20 reliability points
- **ROI**: 20 pts / 3 days = **6.7 pts per day**

---

## Conclusion

**Recommended Start**: Phase 1 Task 1.1 (Parallel Processing)  
**Highest ROI**: P0A (76% gain per day)  
**Critical Path**: P0A → P0B → P0C  
**Total Duration**: 12 weeks to full feature parity

**Next Step**: Begin implementation with `git checkout -b feat/parallel-extraction`
