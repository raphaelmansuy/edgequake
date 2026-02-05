# OODA-42: Orient - Gap Analysis and Prioritization

## Date: 2026-02-05

## Key Finding: Evaluation Already Uses pdfium!

**Critical Insight:** The `eval_comprehensive.py` script runs with `--features pdfium`:

```python
result = subprocess.run(
    ["cargo", "run", "--features", "pdfium", "-p", "edgequake-pdf", ...],
    ...
)
```

This means:

- Current quality score (0.786) reflects the **pdfium pipeline**
- The lopdf pipeline is NOT being evaluated
- The gap to 0.95 target is in the pdfium pipeline itself

---

## Updated Architecture Understanding

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CURRENT STATE                                    │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  DEFAULT COMPILE:                                                      │
│    cargo build                                                         │
│    └─► Uses lopdf (default feature)                                    │
│    └─► Quality: UNKNOWN (not evaluated)                                │
│                                                                        │
│  EVALUATION COMPILE:                                                   │
│    cargo run --features pdfium                                         │
│    └─► Uses pdfium backend                                             │
│    └─► Quality: 0.786 (measured)                                       │
│                                                                        │
│  PROBLEM: Two different pipelines, two different code paths!           │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Gap Analysis

### Gap 1: Quality Below Target (0.786 vs 0.95)

| Metric    | Current | Target | Gap    | Priority |
| --------- | ------- | ------ | ------ | -------- |
| QUALITY   | 0.786   | 0.95   | -0.164 | CRITICAL |
| ROUGE-L   | 0.832   | 0.90   | -0.068 | HIGH     |
| Word F1   | 0.941   | 0.95   | -0.009 | LOW      |
| Structure | 0.417   | 0.80   | -0.383 | HIGH     |
| Format    | 0.659   | 0.70   | -0.041 | MEDIUM   |

**Root Cause Analysis:**

1. **Structure Score (0.417)** - Lowest dimension!
   - Headings: Being detected but at wrong levels
   - Paragraphs: Block grouping issues
   - Lines: Over-fragmentation or over-merging

2. **ROUGE-L (0.832)** - Reading order issues
   - Multi-column detection incomplete
   - Block ordering algorithm needs improvement

### Gap 2: Dual Pipeline Maintenance Burden

| Issue                            | Impact                | Solution                |
| -------------------------------- | --------------------- | ----------------------- |
| 10,086 lines in backend/         | High maintenance cost | Deprecate lopdf modules |
| Two TextGrouper implementations  | DRY violation         | Consolidate to pdfium   |
| Two font style detection methods | Inconsistent results  | Use pdfium flags only   |

### Gap 3: Documentation Debt

| Item            | Current    | Required            |
| --------------- | ---------- | ------------------- |
| OODA iterations | 5 complete | 60+ needed          |
| WHY comments    | Sparse     | Comprehensive       |
| ASCII diagrams  | Few        | More algorithm docs |

---

## Prioritized Action Plan

### Phase 1: Establish Single Source of Truth (OODA 42-45)

1. **Make pdfium the default backend** - Cargo.toml change
2. **Mark lopdf modules as deprecated** - Add warnings
3. **Update extractor.rs** - Clean backend selection

### Phase 2: Quality Improvement (OODA 46-55)

4. **Fix Structure score** - Block grouping improvements
5. **Fix ROUGE-L** - Reading order algorithm
6. **Improve Format score** - Bold/italic edge cases

### Phase 3: Code Cleanup (OODA 56-65)

7. **Apply SRP** - Split pymupdf_grouper.rs (1362 lines)
8. **Apply DRY** - Remove duplicate code
9. **Add WHY comments** - Document decisions
10. **Add ASCII diagrams** - Algorithm visualization

### Phase 4: Documentation (OODA 66-100)

11. Create OODA iterations 42-100
12. Update mission spec
13. Final quality validation

---

## Dependencies and Risks

### Dependencies

1. **libpdfium.dylib** - Must be available at runtime
   - Current: `lib/lib/libpdfium.dylib`
   - Size: ~40MB
   - Mitigate: Bundle with release or document setup

2. **pdfium-render 0.8** - Rust bindings
   - License: MIT OR Apache-2.0 (compatible)
   - API stability: Good

### Risks

| Risk               | Likelihood | Impact | Mitigation                       |
| ------------------ | ---------- | ------ | -------------------------------- |
| Breaking tests     | High       | Medium | Run full suite after each change |
| Quality regression | Medium     | High   | Track metrics per commit         |
| Build complexity   | Low        | Low    | Document pdfium setup            |

---

## Metrics to Track

After each OODA iteration, run:

```bash
# Quick quality check
cd /Users/raphaelmansuy/Github/03-working/edgequake
python3 scripts/eval_comprehensive.py

# Expected format:
# Average QUALITY: X.XXX (target: ≥0.95, gap: +X.XXX)
```

Target progression:

- OODA-42: 0.786 (baseline)
- OODA-50: 0.85
- OODA-60: 0.90
- OODA-70: 0.95

---

## Next Steps (Decide)

Decision needed on:

1. Should we completely remove lopdf or keep as fallback?
2. What's the priority: Quality improvement or code cleanup?
3. How to handle environments without libpdfium?
