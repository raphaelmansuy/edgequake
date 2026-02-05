# OODA Iteration 01 - Orient

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Mission File**: `specs/007-improve-pdf-markdown-quality.md`
**Date**: 2026-02-05

---

## 1. Gap Analysis

### 1.1 Root Cause Analysis (First Principles)

**Problem**: Text appears fragmented and out of order

**Why #1**: Reading order computed after text is already grouped
**Why #2**: Text grouping doesn't consider column boundaries
**Why #3**: Column detection runs separately from text extraction
**Why #4**: No unified pipeline that flows: Parse → Columns → Group → Order → Render
**Why #5**: Legacy architecture evolved incrementally, not designed holistically

**Root Cause**: The extraction pipeline lacks a unified column-aware grouping stage.

### 1.2 Current Pipeline Flow (Observed)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    CURRENT PIPELINE (Problematic)                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  PDF → Parse chars → Group into lines → Group into blocks           │
│                         │                                           │
│                         ↓                                           │
│                   Detect columns (AFTER grouping)                   │
│                         │                                           │
│                         ↓                                           │
│                   Apply reading order                               │
│                         │                                           │
│                         ↓                                           │
│                   Render Markdown                                   │
│                                                                     │
│  PROBLEM: Blocks formed before column context is available!        │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 Desired Pipeline Flow (PyMuPDF4LLM Approach)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    TARGET PIPELINE (PyMuPDF4LLM-style)             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  PDF → Extract blocks with bboxes → Filter margins                 │
│                         │                                           │
│                         ↓                                           │
│              Detect columns from block positions                    │
│                         │                                           │
│                         ↓                                           │
│              Join touching rectangles (3 phases)                    │
│                         │                                           │
│                         ↓                                           │
│              Sort by column-aware key: (left_y, current_x)         │
│                         │                                           │
│                         ↓                                           │
│              Render Markdown with proper reading order              │
│                                                                     │
│  KEY INSIGHT: Column detection BEFORE final grouping!              │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Solution Options Analysis

### Option A: Refactor Pipeline Order (High Impact, High Effort)

**Description**: Reorder pipeline to detect columns before final block grouping.

**Pros**:
- Addresses root cause
- Matches PyMuPDF4LLM architecture
- Long-term maintainability

**Cons**:
- Significant refactoring
- Risk of regression
- 2-3 day effort

**Risk**: Medium (well-tested codebase)

### Option B: Add Header/Footer Margin Filtering (Medium Impact, Low Effort)

**Description**: Add margin parameters to filter noise before processing.

**Pros**:
- Quick win (1-2 hours)
- Immediately improves quality
- Low regression risk

**Cons**:
- Doesn't fix root cause
- Partial improvement only

**Risk**: Low

### Option C: Improve Reading Order Algorithm (Medium Impact, Medium Effort)

**Description**: Enhance `reading_order.rs` to use PyMuPDF4LLM's column-aware sorting.

**Pros**:
- Targeted fix
- Moderate effort
- Can be done incrementally

**Cons**:
- May not fully resolve fragmentation
- Still working with improperly grouped blocks

**Risk**: Low

### Option D: Rectangle Joining Post-Processing (Medium Impact, Medium Effort)

**Description**: Implement 3-phase rectangle joining from PyMuPDF4LLM.

**Pros**:
- Directly addresses block fragmentation
- Well-documented algorithm (from Python)

**Cons**:
- Adds complexity
- May conflict with existing grouping

**Risk**: Medium

---

## 3. Impact-Effort Matrix

```
                    HIGH IMPACT
                         │
              ┌──────────┼──────────┐
              │    A     │    C     │
              │ Pipeline │ Reading  │
              │ Refactor │ Order    │
   HIGH ──────┼──────────┼──────────┤ LOW
   EFFORT     │    D     │    B     │
              │ Rect     │ Margin   │
              │ Joining  │ Filter   │
              └──────────┼──────────┘
                         │
                    LOW IMPACT
```

---

## 4. Recommended Approach (First Principles)

**Phase 1 (Iteration 01-05): Quick Wins**
1. ✅ Add header/footer margin filtering (Option B)
2. ✅ Improve reading order with column-aware sorting (Option C)

**Phase 2 (Iteration 06-15): Core Improvements**
3. Implement rectangle joining algorithm (Option D)
4. Add list bullet detection from visual cues

**Phase 3 (Iteration 16-30): Architecture**
5. Refactor pipeline order (Option A)
6. Implement PyMuPDF4LLM-style block extraction

**Phase 4 (Iteration 31-50): Polish**
7. Table detection improvements
8. Code block detection refinement
9. Performance optimization

---

## 5. Quality Principles Check

| Principle | Current State | Target |
|-----------|---------------|--------|
| SRP | Partial - some modules do too much | Each module one responsibility |
| DRY | Good - minimal duplication | Maintain |
| Dead Code | Some unused paths | Audit and remove |
| Fast Tests | 0.08s for 494 tests | Maintain or improve |
| First Principles | Incremental evolution | Root cause focus |

---

## 6. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in existing tests | Medium | High | Run full test suite after each change |
| Performance degradation | Low | Medium | Benchmark before/after |
| Scope creep | Medium | Medium | Strict OODA iteration boundaries |
| Breaking API | Low | High | No public API changes in Phase 1-2 |

---

## 7. Decision Framework for Next Step

**Immediate Priority**: Option B (Margin Filtering)

**Rationale**:
1. Low risk, high confidence
2. Measurable improvement
3. Sets foundation for further work
4. Can be implemented in one iteration
