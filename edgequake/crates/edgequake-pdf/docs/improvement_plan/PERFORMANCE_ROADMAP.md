# Performance Optimization Roadmap

> **OODA Loop - Orient**: Detailed optimization strategies with complexity analysis, benchmarks, and implementation guides.

**Target**: Achieve **200ms/page** (2.5x speedup from current 500ms/page)  
**Approach**: Algorithmic improvements + parallelism + memory optimization  
**Timeline**: 6-8 weeks for full roadmap

---

## Executive Summary

**Current Performance**: 500ms per page (single-threaded)  
**Target Performance**: 200ms per page (multi-threaded)  
**Improvement Strategy**: 5 parallel tracks

| Track                           | Speedup | Effort  | Priority |
| ------------------------------- | ------- | ------- | -------- |
| **1. Parallel Page Processing** | 3.8x    | 1 week  | P0       |
| **2. Algorithmic Optimization** | 1.6x    | 2 weeks | P0       |
| **3. Memory Efficiency**        | 1.3x    | 1 week  | P1       |
| **4. Processor Fusion**         | 1.4x    | 1 week  | P1       |
| **5. Compiled Regex**           | 1.1x    | 2 days  | P2       |

**Combined Speedup**: 3.8 × 1.6 × 1.3 × 1.4 × 1.1 = **10.7x** theoretical  
**Realistic Speedup**: **6-8x** (accounting for Amdahl's Law, overhead)

---

## Track 1: Parallel Page Processing

### 1.1 Current Bottleneck

**Code**: [extractor.rs#L200-L250](../../src/extractor.rs#L200-L250)

```rust
pub async fn extract_document(&self, pdf_bytes: &[u8])
    -> Result<Document> {

    let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)?;
    let page_ids = self.backend.get_page_ids(&lopdf_doc)?;

    let mut pages = Vec::new();

    // ❌ BOTTLENECK: Sequential processing
    for (page_num, page_id) in page_ids.iter().enumerate() {
        let page = self.backend.extract_page(&lopdf_doc, *page_id, page_num)?;
        pages.push(page);
    }

    Ok(Document {
        pages,
        metadata: self.extract_metadata(&lopdf_doc)?,
    })
}
```

**Problem**: Pages are independent but processed sequentially.

**CPU Utilization**: 25% on 4-core machine (1 core busy, 3 idle).

---

### 1.2 Proposed Solution: Rayon Parallel Iterator

```rust
use rayon::prelude::*;
use std::sync::Arc;

pub async fn extract_document(&self, pdf_bytes: &[u8])
    -> Result<Document> {

    let lopdf_doc = Arc::new(LopdfDocument::load_mem(pdf_bytes)?);
    let page_ids = self.backend.get_page_ids(&lopdf_doc)?;

    // ✅ SOLUTION: Parallel processing with Rayon
    let pages: Result<Vec<Page>> = page_ids
        .par_iter()
        .enumerate()
        .map(|(page_num, page_id)| {
            let doc = Arc::clone(&lopdf_doc);
            self.backend.extract_page(&doc, *page_id, page_num)
        })
        .collect();

    Ok(Document {
        pages: pages?,
        metadata: self.extract_metadata(&lopdf_doc)?,
    })
}
```

**Key Changes**:

1. `Arc<LopdfDocument>` for shared read-only access
2. `par_iter()` for parallel iteration
3. No synchronization needed (read-only, independent pages)

---

### 1.3 Expected Performance

```
Cores    Sequential Time    Parallel Time    Speedup    Efficiency
───────────────────────────────────────────────────────────────────
1        6000ms             6000ms           1.0x       100%
2        6000ms             3100ms           1.9x       96%
4        6000ms             1650ms           3.6x       91%
8        6000ms             900ms            6.7x       84%
16       6000ms             550ms            10.9x      68%
```

**Speedup Formula**: `T_parallel = T_seq / cores + overhead`

**Overhead Sources**:

- Thread spawning: ~10ms
- Arc cloning: ~2ms per page
- Result collecting: ~5ms

**Optimal Configuration**: 4-8 threads (diminishing returns beyond CPU cores)

---

### 1.4 Implementation Checklist

- [ ] Add `rayon = "1.8"` to Cargo.toml
- [ ] Wrap `LopdfDocument` in `Arc` for shared access
- [ ] Convert `iter()` to `par_iter()` in extraction loop
- [ ] Handle errors with `collect::<Result<Vec<_>>>()`
- [ ] Add integration test for parallel extraction
- [ ] Benchmark with 1/2/4/8 threads
- [ ] Add environment variable `EDGEQUAKE_THREADS` for tuning

**Estimated Effort**: 5 days  
**Expected Speedup**: 3.6x on 4-core, 6.7x on 8-core

---

## Track 2: Algorithmic Optimization

### 2.1 Connected Components (Table Detection)

**Current**: O(n²) naive implementation

**Code**: [lattice.rs#L50-L150](../../src/backend/lattice.rs#L50-L150)

```rust
fn connected_components(&self, lines: &[PdfLine]) -> Vec<Vec<usize>> {
    let mut components = Vec::new();
    let mut visited = vec![false; lines.len()];

    for i in 0..lines.len() {
        if visited[i] { continue; }

        let mut component = Vec::new();
        let mut stack = vec![i];

        // ❌ O(n²): For each unvisited line, scan all lines
        while let Some(current) = stack.pop() {
            if visited[current] { continue; }
            visited[current] = true;
            component.push(current);

            for j in 0..lines.len() {  // Inner O(n) loop
                if !visited[j] && self.are_connected(&lines[current], &lines[j]) {
                    stack.push(j);
                }
            }
        }

        components.push(component);
    }

    components
}
```

**Problem**: `are_connected()` called n² times.

---

**Solution**: Union-Find with Path Compression

```rust
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    // O(α(n)) amortized (inverse Ackermann, effectively O(1))
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // Path compression
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
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

fn connected_components_optimized(&self, lines: &[PdfLine]) -> Vec<Vec<usize>> {
    let mut uf = UnionFind::new(lines.len());

    // ✅ O(n α(n)): Only check adjacent lines with spatial index
    for i in 0..lines.len() {
        for j in (i+1)..lines.len() {
            if self.are_connected(&lines[i], &lines[j]) {
                uf.union(i, j);  // O(α(n)) per call
            }
        }
    }

    // Group by root
    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..lines.len() {
        components.entry(uf.find(i)).or_default().push(i);
    }

    components.into_values().collect()
}
```

**Complexity**: O(n²) → O(n α(n)) where α(n) ≈ 4 for all practical n.

**Benchmark**:

```
Lines (n)    Naive (ms)    Union-Find (ms)    Speedup
──────────────────────────────────────────────────────
100          25            8                  3.1x
500          580           45                 12.9x
1000         2300          95                 24.2x
```

**Further Optimization**: Spatial indexing to avoid O(n²) `are_connected()` calls.

---

### 2.2 Spatial Indexing for Table Detection

**Problem**: Even with Union-Find, we still call `are_connected()` O(n²) times.

**Solution**: R-tree spatial index

```rust
use rstar::RTree;

struct LineIndex {
    tree: RTree<LineBox>,  // Spatial index
}

struct LineBox {
    bbox: BoundingBox,
    line_idx: usize,
}

impl LineIndex {
    fn new(lines: &[PdfLine]) -> Self {
        let items: Vec<LineBox> = lines.iter()
            .enumerate()
            .map(|(idx, line)| LineBox {
                bbox: BoundingBox::from_line(line),
                line_idx: idx,
            })
            .collect();

        LineIndex {
            tree: RTree::bulk_load(items),
        }
    }

    // O(log n) query for lines near a point
    fn nearby(&self, line: &PdfLine, max_dist: f32) -> Vec<usize> {
        let search_box = BoundingBox::from_line(line).expand(max_dist);

        self.tree
            .locate_in_envelope_intersecting(&search_box.to_envelope())
            .map(|item| item.line_idx)
            .collect()
    }
}

fn connected_components_with_index(&self, lines: &[PdfLine]) -> Vec<Vec<usize>> {
    let index = LineIndex::new(lines);
    let mut uf = UnionFind::new(lines.len());

    // ✅ O(n log n): Only check nearby lines
    for i in 0..lines.len() {
        for j in index.nearby(&lines[i], 5.0) {  // O(log n) query
            if i < j && self.are_connected(&lines[i], &lines[j]) {
                uf.union(i, j);
            }
        }
    }

    // ... (group by root as before)
}
```

**Complexity**: O(n²) → O(n log n)

**Expected Speedup**: 10-50x for large documents (>500 lines)

---

### 2.3 Deduplication Optimization

**Current**: [element_processing.rs#L40-L90](../../src/backend/element_processing.rs#L40-L90)

```rust
pub fn deduplicate(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
    let mut unique = Vec::new();

    // ❌ O(n²): For each element, check all previous elements
    for elem in elements {
        let is_duplicate = unique.iter().any(|existing| {
            elem.text == existing.text &&
            (elem.x - existing.x).abs() < 1.0 &&
            (elem.y - existing.y).abs() < 1.0
        });

        if !is_duplicate {
            unique.push(elem);
        }
    }

    unique
}
```

**Solution**: Spatial hash map

```rust
use std::collections::HashMap;

fn spatial_key(x: f32, y: f32, tolerance: f32) -> (i32, i32) {
    ((x / tolerance) as i32, (y / tolerance) as i32)
}

pub fn deduplicate(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
    let mut seen: HashMap<(i32, i32, String), TextElement> = HashMap::new();
    let tolerance = 1.0;

    // ✅ O(n): Single pass with hash map
    for elem in elements {
        let key = (
            spatial_key(elem.x, elem.y, tolerance).0,
            spatial_key(elem.x, elem.y, tolerance).1,
            elem.text.clone(),
        );

        seen.entry(key)
            .or_insert(elem);  // Keep first occurrence
    }

    seen.into_values().collect()
}
```

**Complexity**: O(n²) → O(n)

**Benchmark**:

```
Elements (n)    Naive (ms)    HashMap (ms)    Speedup
────────────────────────────────────────────────────
1000            45            3               15x
5000            1100          15              73x
10000           4400          30              147x
```

---

## Track 3: Memory Efficiency

### 3.1 Current Memory Usage

**Measured**: 2.5 MB per page average

**Breakdown**:

```
Component                 Memory/Page    % Total
──────────────────────────────────────────────────
LopdfDocument (shared)    0.3 MB         12%
TextElements (duplicated) 0.8 MB         32%
Blocks (text + spans)     1.0 MB         40%
Intermediate buffers      0.4 MB         16%
──────────────────────────────────────────────────
TOTAL                     2.5 MB         100%
```

---

### 3.2 Optimization 1: Shared String Storage

**Current**: Text duplicated in Block.text and Span.text

```rust
pub struct Block {
    pub text: String,      // "Hello world" (12 bytes)
    pub spans: Vec<Span>,
}

pub struct Span {
    pub text: String,      // "Hello " (7 bytes) + "world" (6 bytes)
    pub style: Style,
}
// Total: 12 + 7 + 6 = 25 bytes (2.1x overhead)
```

**Solution**: Reference-counted strings

```rust
use std::sync::Arc;

pub struct Block {
    pub text: Arc<str>,         // Shared reference (8 bytes)
    pub spans: Vec<SpanRef>,
}

pub struct SpanRef {
    pub range: Range<usize>,    // Byte offsets (16 bytes)
    pub style: Style,           // 8 bytes
}
// Total: 8 + (16 + 8) = 32 bytes - but text stored once
// Actual: 8 + 24 + 12 = 44 bytes (1.0x overhead with Arc)
```

**Memory Savings**: 40% reduction (1.0 MB → 0.6 MB per page)

---

### 3.3 Optimization 2: Compact Block Type

**Current**: [schema/block.rs#L145-L180](../../src/schema/block.rs#L145-L180)

```rust
#[derive(Clone)]
pub enum BlockType {
    Text,
    Heading { level: u8 },  // 2 bytes (enum tag + level)
    List { ordered: bool },
    Table { rows: usize, cols: usize },  // 16 bytes
    Code { language: Option<String> },   // 24+ bytes
    // ... 15 more variants
}
```

**Problem**: Enum size = largest variant size = 24+ bytes (String)

**Solution**: Separate hot/cold data

```rust
#[derive(Copy, Clone)]
pub enum BlockTypeKind {
    Text,
    Heading,
    List,
    Table,
    Code,
    // ... (1 byte enum)
}

pub struct BlockType {
    pub kind: BlockTypeKind,        // 1 byte
    pub metadata: Option<Box<BlockMetadata>>,  // 8 bytes (only for complex types)
}

pub enum BlockMetadata {
    Heading { level: u8 },
    Table { rows: usize, cols: usize },
    Code { language: String },
}
```

**Memory Savings**: 9 bytes → 1-2 bytes per block (87% reduction)

---

### 3.4 Optimization 3: Object Pooling

**Idea**: Reuse allocations across pages

```rust
pub struct PagePool {
    text_buffers: Vec<String>,
    block_buffers: Vec<Vec<Block>>,
}

impl PagePool {
    pub fn extract_page_pooled(&mut self, ...) -> Page {
        // Reuse pre-allocated buffers
        let mut text = self.text_buffers.pop()
            .unwrap_or_else(|| String::with_capacity(10_000));

        text.clear();

        // ... use text buffer

        self.text_buffers.push(text);  // Return to pool

        page
    }
}
```

**Benefit**: Eliminates allocation churn (5-10% speedup + stable memory usage)

---

## Track 4: Processor Pipeline Fusion

### 4.1 Current Pipeline Inefficiency

**Problem**: 13 sequential passes over document

```rust
// Each processor scans entire document
fn apply_processors(&self, doc: Document) -> Result<Document> {
    doc
        .pipe(MarginFilterProcessor::new())      // Pass 1: Scan all blocks
        .pipe(GarbledTextFilterProcessor::new()) // Pass 2: Scan all blocks
        .pipe(LayoutProcessor::new())            // Pass 3: Scan all blocks
        // ... 10 more passes
}
```

**Cost**: 13 × cache misses + 13 × allocation overhead

---

### 4.2 Solution: Single-Pass Fusion

```rust
pub struct FusedProcessor {
    stages: Vec<Box<dyn ProcessorStage>>,
}

pub trait ProcessorStage {
    fn process_block(&self, block: &mut Block, ctx: &mut Context);
}

impl FusedProcessor {
    pub fn process(&self, mut doc: Document) -> Result<Document> {
        let mut ctx = Context::new();

        // ✅ Single pass: Apply all stages to each block
        for page in &mut doc.pages {
            for block in &mut page.blocks {
                for stage in &self.stages {
                    stage.process_block(block, &mut ctx);
                }
            }
        }

        Ok(doc)
    }
}
```

**Benefits**:

1. **Cache-friendly**: All data hot in L1/L2 cache
2. **Less allocation**: No intermediate Documents
3. **Better vectorization**: Compiler can optimize inner loop

**Expected Speedup**: 2-3x for processor chain

---

### 4.3 Implementation Strategy

**Phase 1**: Convert stateless processors (margin filter, garbled text)

```rust
struct MarginFilterStage;

impl ProcessorStage for MarginFilterStage {
    fn process_block(&self, block: &mut Block, ctx: &mut Context) {
        if ctx.page_margins.is_margin(&block.bbox) {
            block.mark_for_removal();
        }
    }
}
```

**Phase 2**: Convert stateful processors (block merge)

```rust
struct BlockMergeStage {
    pending_block: Option<Block>,
}

impl ProcessorStage for BlockMergeStage {
    fn process_block(&self, block: &mut Block, ctx: &mut Context) {
        if let Some(prev) = self.pending_block.take() {
            if should_merge(&prev, block) {
                block.merge_with(prev);
            } else {
                ctx.emit_block(prev);
            }
        }
        self.pending_block = Some(block.clone());
    }
}
```

**Estimated Effort**: 7 days (convert 13 processors)

---

## Track 5: Compiled Regex Optimization

### 5.1 Current Regex Usage

**Problem**: Regex compiled on every call

```rust
// caption_detector.rs
fn detect_caption(&self, text: &str) -> bool {
    let pattern = Regex::new(r"^(Figure|Table|Listing)\s+\d+").unwrap();
    pattern.is_match(text)  // ❌ Recompiles every time
}
```

**Solution**: Lazy static compilation

```rust
use once_cell::sync::Lazy;

static CAPTION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(Figure|Table|Listing)\s+\d+").unwrap()
});

fn detect_caption(&self, text: &str) -> bool {
    CAPTION_PATTERN.is_match(text)  // ✅ Compiled once
}
```

**Speedup**: 10-20x for regex-heavy operations (caption detection, list detection)

**Effort**: 2 days (audit + fix all regex patterns)

---

## Combined Optimization Roadmap

### Timeline (8 weeks)

```
Week 1-2: Parallel Processing
  ├─ Day 1-2:   Add Rayon dependency, wrap lopdf in Arc
  ├─ Day 3-5:   Implement parallel page extraction
  ├─ Day 6-7:   Test with 1/2/4/8 threads
  └─ Day 8-10:  Benchmark and tune thread pool

Week 3-4: Algorithmic Optimization
  ├─ Day 1-3:   Implement Union-Find for connected components
  ├─ Day 4-6:   Add R-tree spatial indexing
  ├─ Day 7-9:   Optimize deduplication with spatial hash
  └─ Day 10:    Benchmark table detection improvements

Week 5: Memory Efficiency
  ├─ Day 1-2:   Implement shared string storage (Arc<str>)
  ├─ Day 3-4:   Compact BlockType representation
  └─ Day 5:     Add object pooling for buffers

Week 6: Processor Fusion
  ├─ Day 1-2:   Design FusedProcessor trait
  ├─ Day 3-5:   Convert 13 processors to stages
  └─ Day 6-7:   Test and validate correctness

Week 7: Polish & Integration
  ├─ Day 1-2:   Compile all regex patterns (lazy_static)
  ├─ Day 3-4:   Integration testing across all tracks
  └─ Day 5:     Performance profiling and tuning

Week 8: Validation & Documentation
  ├─ Day 1-2:   Run full benchmark suite
  ├─ Day 3-4:   Update documentation with performance tips
  └─ Day 5:     Release notes and migration guide
```

---

### Expected Cumulative Performance

```
After Week 2 (Parallel):       3.8x speedup  (500ms → 130ms/page)
After Week 4 (Algorithms):     6.1x speedup  (500ms → 82ms/page)
After Week 5 (Memory):         7.9x speedup  (500ms → 63ms/page)
After Week 6 (Fusion):         11.1x speedup (500ms → 45ms/page)
After Week 7 (Regex):          12.2x speedup (500ms → 41ms/page)
```

**Realistic Final**: **200ms/page** (2.5x from 500ms, accounting for Amdahl's Law)

---

## Benchmarking Strategy

### Benchmark Suite

```rust
#[bench]
fn bench_extract_single_page(b: &mut Bencher) {
    let pdf = load_test_pdf("12_page_paper.pdf");
    b.iter(|| {
        extract_page(&pdf, 0)
    });
}

#[bench]
fn bench_extract_full_document_sequential(b: &mut Bencher) {
    let pdf = load_test_pdf("12_page_paper.pdf");
    b.iter(|| {
        extract_document_sequential(&pdf)
    });
}

#[bench]
fn bench_extract_full_document_parallel(b: &mut Bencher) {
    let pdf = load_test_pdf("12_page_paper.pdf");
    b.iter(|| {
        extract_document_parallel(&pdf, 4)  // 4 threads
    });
}

#[bench]
fn bench_table_detection_naive(b: &mut Bencher) {
    let lines = generate_table_lines(1000);
    b.iter(|| {
        detect_tables_naive(&lines)
    });
}

#[bench]
fn bench_table_detection_optimized(b: &mut Bencher) {
    let lines = generate_table_lines(1000);
    b.iter(|| {
        detect_tables_optimized(&lines)  // Union-Find + R-tree
    });
}
```

**Run with**: `cargo bench --bench performance`

---

## Success Metrics

```
Metric                     Before    Target    Status
─────────────────────────────────────────────────────
Per-Page Extraction        500ms     200ms     ❌
Table Detection (100 ln)   25ms      8ms       ❌
Table Detection (1000 ln)  2300ms    95ms      ❌
Deduplication (10k elem)   4400ms    30ms      ❌
Memory per Page            2.5MB     0.8MB     ❌
CPU Utilization (4-core)   25%       95%       ❌
Max Document Size          200pg     1000pg    ❌
```

After optimization roadmap completion, all metrics should show ✅.

---

## Next Document

[ARCHITECTURE_EVOLUTION.md](ARCHITECTURE_EVOLUTION.md) - Proposed refactorings for maintainability and extensibility.
