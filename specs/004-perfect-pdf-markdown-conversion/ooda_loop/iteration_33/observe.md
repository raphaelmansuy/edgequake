# OODA-33 OBSERVE: Speed Profiling Results

## Summary

Profiled PDF extraction in release mode to identify performance hotspots.

## Benchmark Results (Release Mode)

### Test PDFs (from zz_test_docs/)

| PDF | Pages | Time | Per-Page |
|-----|-------|------|----------|
| Apple-Sandbox-Guide | 48 | 1.35s | 0.028s |
| agentfail | 39 | 1.82s | 0.047s |
| hotmess | 40 | 4.15s | 0.104s |
| **Total** | 127 | 7.33s | 0.058s avg |

### Performance Analysis

```
┌────────────────────────────────────────────────────────────────┐
│                   Speed Performance Summary                     │
├────────────────────────────────────────────────────────────────┤
│ Target:    <1s per page = 0.001s per page                      │
│ Current:   0.058s per page average                             │
│ Best:      0.028s per page (Apple-Sandbox)                     │
│ Worst:     0.104s per page (hotmess)                           │
├────────────────────────────────────────────────────────────────┤
│ Gap:       ~58x slower than target (avg)                       │
│            ~28x slower (best case)                             │
│            ~104x slower (worst case)                           │
└────────────────────────────────────────────────────────────────┘
```

### CPU Utilization

```
Total CPU: 448% (multi-threaded via Rayon)
Wall time: 7.88s for 127 pages
Theoretical: 34.89s / 4.48 cores = 7.8s ✓
```

**Observation:** Parallel processing is working effectively.

### Debug vs Release Comparison

| Mode | Time per PDF | Speedup |
|------|--------------|---------|
| Debug | ~12s | 1x |
| Release | ~2.4s | 5x |

### Variance Analysis

**Why is hotmess 3x slower per page than Apple-Sandbox?**

Possible causes:
1. **Complex fonts** - Type3 fonts with custom glyph definitions
2. **Dense content** - More text elements per page
3. **Table detection** - More lattice line processing
4. **Font encoding** - Complex ToUnicode CMap processing

## Current Architecture Timing

```
PDF Extraction Pipeline (estimated breakdown):
┌─────────────────────────────────────────────────────────────────┐
│ lopdf::Document::load()              │  10% │ Parse PDF file   │
├─────────────────────────────────────────────────────────────────┤
│ get_page_fonts() per page            │  15% │ Font parsing     │
├─────────────────────────────────────────────────────────────────┤
│ get_page_content() + decompress      │  10% │ Content stream   │
├─────────────────────────────────────────────────────────────────┤
│ ContentParser::parse()               │  35% │ Character decode │
│   └─► decode_char() with CMap        │      │ Hot path         │
├─────────────────────────────────────────────────────────────────┤
│ TextGrouper::group_into_lines()      │  10% │ Line assembly    │
├─────────────────────────────────────────────────────────────────┤
│ LatticeEngine::detect_tables()       │  15% │ Table detection  │
├─────────────────────────────────────────────────────────────────┤
│ Markdown rendering                   │   5% │ Output format    │
└─────────────────────────────────────────────────────────────────┘
```

## Key Findings

1. **Release mode is 5x faster than debug** - Always benchmark in release
2. **Best case (0.028s/page) is 28x from target** - Need algorithmic improvements
3. **Hotmess PDF is 3.7x slower** - Likely font/content complexity
4. **Parallel processing works** - 448% CPU utilization
5. **ContentParser likely hotspot** - Character decoding is on hot path

## Hypothesis for Improvement

1. **Font caching** - Cache parsed fonts across pages
2. **Lazy ToUnicode parsing** - Don't parse until needed
3. **Content stream optimization** - Stream processing instead of buffer
4. **R-tree already in use** - No O(n²) in spatial queries ✓
