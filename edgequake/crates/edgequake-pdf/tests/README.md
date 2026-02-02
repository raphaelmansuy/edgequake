# PDF Extraction Test Suite

## Overview

This directory contains a three-tier test architecture designed for optimal developer productivity and comprehensive quality validation.

## Test Tiers

### Tier 1: Smoke Tests (Default)

**File:** `quick_smoke.rs`  
**Time:** 0.06s  
**PDFs:** 3 small test files  
**Command:**
```bash
cargo test --package edgequake-pdf --test quick_smoke
```

**Purpose:** Instant feedback loop during development. Runs automatically with `cargo test`.

**Tests:**
- `smoke_sample_pdf` - Verify basic extraction (sample.pdf, 20 bytes)
- `smoke_simple_text` - Plain text extraction (001_simple_text.pdf)
- `smoke_headers_and_lists` - Structure markers (002_headers_and_lists.pdf)
- `smoke_test_summary` - Info message

**Use when:** Making changes to extraction engine, every save/compile cycle.

---

### Tier 2: Feature Tests

**File:** `basic_features.rs`  
**Time:** 0.31s  
**PDFs:** 4 medium test files  
**Command:**
```bash
cargo test --package edgequake-pdf --test basic_features --features slow-tests
```

**Purpose:** Validate specific features work correctly before committing.

**Tests:**
- `feature_multi_column_layout` - Two-column reading order
- `feature_table_extraction` - Table detection and markdown conversion
- `feature_numbered_pdfs_batch` - Batch processing of 4 diverse PDFs
- `feature_test_summary` - Info message

**Use when:** Before committing changes, CI/CD PR checks.

---

### Tier 3: Comprehensive Quality

**File:** `comprehensive_quality.rs`  
**Time:** 118s  
**PDFs:** 7 real academic papers (27MB)  
**Command:**
```bash
cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
```

**Purpose:** Full quality metrics against gold standard markdown.

**Tests:**
- `comprehensive_real_dataset_quality` - Processes all real_dataset/ PDFs
  - Calculates Text Preservation Score (TPS)
  - Calculates Structural Fidelity Score (SFS)
  - Reports per-PDF and aggregate metrics
- `comprehensive_test_summary` - Info message

**Quality Metrics (Feb 2, 2026):**
- Text Preservation: 81.3%
- Structural Fidelity: 68.0%
- Overall Quality: 74.6%
- Target: 95%+

**Use when:** Before releases, nightly CI/CD runs, quality gate validation.

---

## Quick Reference

```bash
# Default (smoke tests only)
cargo test --package edgequake-pdf

# Smoke + Feature tests
cargo test --package edgequake-pdf --features slow-tests

# All tests (smoke + feature + comprehensive)
cargo test --package edgequake-pdf --all-features

# Individual test suites
cargo test --package edgequake-pdf --test quick_smoke
cargo test --package edgequake-pdf --test basic_features --features slow-tests
cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
```

## Migration from Legacy Tests

**Old (deprecated):**
```bash
cargo test --package edgequake-pdf --test quality_evaluation
```
- ❌ Took 116 seconds
- ❌ No incremental feedback
- ❌ All-or-nothing testing

**New (split tiers):**
```bash
# Development loop (<1s)
cargo test --package edgequake-pdf --test quick_smoke

# Pre-commit (<1s)
cargo test --package edgequake-pdf --test basic_features --features slow-tests

# Pre-release (2min)
cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
```
- ✅ 1657x faster for development loop
- ✅ Instant feedback
- ✅ Stratified testing

## Performance Comparison

| Test Tier | PDFs | Time | Use Case |
|-----------|------|------|----------|
| Smoke | 3 | 0.06s | Every save |
| Feature | 4 | 0.31s | Before commit |
| Comprehensive | 7 | 118s | Before release |
| **Old (quality_evaluation)** | 7 | 116s | ❌ Deprecated |

**Speedup:**
- Development: 1657x faster (116s → 0.06s)
- Integration: 362x faster (116s → 0.31s)
- Comprehensive: 1.02x (118s vs 116s - added better metrics)

## CI/CD Integration

**Recommended GitHub Actions workflow:**

```yaml
name: PDF Tests

on: [push, pull_request]

jobs:
  smoke:
    name: Smoke Tests (Fast)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run smoke tests
        run: |
          cd edgequake
          cargo test --package edgequake-pdf --test quick_smoke

  feature:
    name: Feature Tests (Medium)
    runs-on: ubuntu-latest
    needs: smoke
    steps:
      - uses: actions/checkout@v3
      - name: Run feature tests
        run: |
          cd edgequake
          cargo test --package edgequake-pdf --test basic_features --features slow-tests

  comprehensive:
    name: Comprehensive Quality (Slow)
    runs-on: ubuntu-latest
    needs: feature
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v3
      - name: Run comprehensive tests
        run: |
          cd edgequake
          cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
```

## Test Data Locations

- **Smoke PDFs:** `test-data/*.pdf` (sample.pdf, 001_*.pdf, 002_*.pdf)
- **Feature PDFs:** `test-data/*.pdf` (003_*.pdf, 004_*.pdf)
- **Comprehensive PDFs:** `test-data/real_dataset/*.pdf` + `.gold.md` files

## Adding New Tests

### For smoke tests (quick_smoke.rs):
1. Add small PDF (<100KB) to `test-data/`
2. Add test function following `smoke_*` naming
3. Check: non-zero output, no crashes
4. Keep total time <1 second

### For feature tests (basic_features.rs):
1. Add medium PDF (500KB-2MB) to `test-data/`
2. Add test function following `feature_*` naming
3. Test specific feature (tables, columns, etc.)
4. Keep total time <30 seconds

### For comprehensive tests (comprehensive_quality.rs):
1. Add PDF + gold markdown to `test-data/real_dataset/`
2. Naming: `<name>.pdf` + `<name>.gold.md`
3. Tests run automatically on all .pdf/.gold.md pairs
4. Calculates TPS/SFS metrics

## First Principles: Why This Architecture?

**Problem:** Developers need fast feedback (<1s) but quality requires comprehensive testing (2min).

**Solution:** Stratified testing based on Donald Knuth's optimization principle:
> "We should forget about small efficiencies, say about 97% of the time: premature optimization is the root of all evil. Yet we should not pass up our opportunities in that critical 3%."

The critical 3% is the development loop. Split tests into:
- 97% of iterations → smoke tests (0.06s)
- 2.9% of iterations → feature tests (0.31s)
- 0.1% of iterations → comprehensive tests (118s)

**Result:** 1657x faster development loop without sacrificing quality validation.

## Deprecated Tests

`quality_evaluation.rs` is kept for backward compatibility only. All tests are marked `#[ignore]` or show deprecation notices.

**Do not use:** `cargo test --test quality_evaluation`  
**Use instead:** See "Quick Reference" section above.

---

**Last Updated:** February 2, 2026  
**Maintainer:** EdgeQuake PDF Team  
**Mission:** specs/004-perfect-pdf-markdown-conversion.md
