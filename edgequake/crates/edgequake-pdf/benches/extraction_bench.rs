//! Extraction benchmarks for edgequake-pdf.
//!
//! Run with: `cargo bench --package edgequake-pdf`
//!
//! Measures:
//! - Symbol map lookup performance
//! - Formula detection overhead
//! - BoundingBox operations
//! - Layout algorithm performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use edgequake_pdf::{
    formula::{FormulaConfig, FormulaDetector, MATH_SYMBOL_MAP},
    Block, BlockType, BoundingBox, Page, Point,
};

/// Benchmark symbol map lookup operations.
fn bench_symbol_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("symbol_map");

    // Common math symbols to look up
    let symbols: Vec<char> = "αβγδεζηθικλμνξπρστυφχψωΣ∫∂∇√∞".chars().collect();

    group.bench_function("lookup_existing", |b| {
        b.iter(|| {
            for &sym in &symbols {
                black_box(MATH_SYMBOL_MAP.get(&sym));
            }
        })
    });

    group.bench_function("lookup_missing", |b| {
        let ascii: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
        b.iter(|| {
            for &c in &ascii {
                black_box(MATH_SYMBOL_MAP.get(&c));
            }
        })
    });

    group.finish();
}

/// Benchmark bounding box operations.
fn bench_bounding_box(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounding_box");

    let bbox1 = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
    let bbox2 = BoundingBox::new(50.0, 25.0, 150.0, 75.0);

    group.bench_function("intersects", |b| {
        b.iter(|| black_box(bbox1.intersects(&bbox2)))
    });

    group.bench_function("overlap_area", |b| {
        b.iter(|| black_box(bbox1.intersection_area(&bbox2)))
    });

    group.bench_function("union", |b| b.iter(|| black_box(bbox1.union(&bbox2))));

    group.bench_function("contains", |b| {
        let point = Point::new(50.0, 25.0);
        b.iter(|| black_box(bbox1.contains_point(&point)))
    });

    group.finish();
}

/// Benchmark formula detection.
fn bench_formula_detection(c: &mut Criterion) {
    use edgequake_pdf::schema::{BlockId, ExtractionMethod, PageStats};
    use std::collections::HashMap;

    let mut group = c.benchmark_group("formula_detection");

    // Create a realistic page with mixed content
    fn make_block(text: &str, x: f32, y: f32) -> Block {
        Block {
            id: BlockId::generate(),
            block_type: BlockType::Text,
            bbox: BoundingBox::new(x, y, x + 200.0, y + 20.0),
            page: 0,
            position: 0,
            text: text.to_string(),
            html: None,
            spans: vec![],
            children: vec![],
            confidence: 1.0,
            level: None,
            source: None,
            metadata: HashMap::new(),
        }
    }

    fn make_page(block_count: usize) -> Page {
        let mut blocks = Vec::with_capacity(block_count);
        for i in 0..block_count {
            let y = (i as f32) * 25.0;
            // Alternate between regular text and math
            let text = if i % 3 == 0 {
                "This is regular text without any math symbols."
            } else if i % 3 == 1 {
                "The equation ∑ αβγ ∫ δε = ∂/∂x shows integration."
            } else {
                "More regular text with some numbers 123.456."
            };
            blocks.push(make_block(text, 50.0, y));
        }

        Page {
            number: 1,
            width: 612.0,
            height: 792.0,
            blocks,
            method: ExtractionMethod::Native,
            stats: PageStats::default(),
            columns: vec![],
            margins: None,
            metadata: HashMap::new(),
        }
    }

    let detector = FormulaDetector::new(FormulaConfig::default());

    for size in [10, 50, 100, 500].iter() {
        let page = make_page(*size);
        group.bench_with_input(BenchmarkId::new("detect_formulas", size), &page, |b, p| {
            b.iter(|| black_box(detector.detect_formulas(p)))
        });
    }

    group.finish();
}

/// Benchmark math density calculation.
fn bench_math_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("math_density");

    let texts = vec![
        ("short_no_math", "Hello world"),
        ("short_with_math", "α + β = γ"),
        ("medium_no_math", "This is a longer piece of text without any mathematical symbols at all."),
        ("medium_with_math", "The integral ∫ f(x) dx from a to b equals F(b) - F(a) where ∂F/∂x = f."),
        ("long_no_math", "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris."),
    ];

    for (name, text) in texts {
        group.bench_with_input(BenchmarkId::new("calculate", name), text, |b, t| {
            b.iter(|| {
                let math_count = t
                    .chars()
                    .filter(|c| MATH_SYMBOL_MAP.contains_key(c))
                    .count();
                let total = t.chars().count();
                black_box(if total > 0 {
                    math_count as f32 / total as f32
                } else {
                    0.0
                })
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_symbol_map,
    bench_bounding_box,
    bench_formula_detection,
    bench_math_density,
);
criterion_main!(benches);
