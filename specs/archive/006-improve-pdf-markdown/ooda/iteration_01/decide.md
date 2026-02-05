# Iteration 01: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Prioritized Action Plan

### Iteration 01 Focus: Quick Wins + Foundation

Based on the Orient analysis, I will execute these changes in priority order:

### 1. Fix Clippy Warnings in Binary Tools (P1)

**Target files**:

- `bin/diagnose_fonts.rs` - collapsible_match
- `bin/test_decode.rs` - collapsible_match
- `bin/debug_page1.rs` - unused import, dead_code
- `bin/trace_content.rs` - unused_variables

**Decision**: Fix all warnings to achieve zero-warning build.

### 2. Add WHY Comments to pdfium.rs Font Detection (P1)

**Target**: `backend/pdfium.rs:163-178`

**Decision**: Add comprehensive comments explaining:

- Why we use `font_is_italic()` instead of name patterns
- Why weight >= 700 is the bold threshold (CSS convention)
- Why spaces inherit style from previous character

### 3. Add ASCII Diagram to pymupdf_grouper.rs (P2)

**Target**: `layout/pymupdf_grouper.rs`

**Decision**: Add algorithm visualization at module level explaining:

- Chars → Spans grouping logic
- Spans → Lines grouping logic
- Lines → Blocks grouping logic

### 4. Strengthen lopdf Deprecation Warning (P2)

**Target**: `backend/extraction_engine.rs`

**Decision**:

- Enhance deprecation message with migration steps
- Add compile-time warning when lopdf feature is used
- Update Cargo.toml to remove lopdf from default features

---

## Implementation Order

```
┌────────────────────────────────────────────────────────────────┐
│                    ITERATION 01 CHANGES                        │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Step 1: Fix clippy warnings (bin/*.rs)                        │
│  ▼                                                             │
│  Step 2: Add WHY comments (pdfium.rs)                          │
│  ▼                                                             │
│  Step 3: Add ASCII diagram (pymupdf_grouper.rs)                │
│  ▼                                                             │
│  Step 4: Update deprecation (extraction_engine.rs)             │
│  ▼                                                             │
│  Step 5: Verify: cargo clippy -p edgequake-pdf → 0 warnings    │
│  ▼                                                             │
│  Step 6: Verify: cargo test -p edgequake-pdf → all pass        │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## Success Criteria for This Iteration

- [ ] Zero clippy warnings in edgequake-pdf crate
- [ ] WHY comments added to pdfium.rs font detection
- [ ] ASCII diagram added to pymupdf_grouper.rs
- [ ] Enhanced deprecation warning in extraction_engine.rs
- [ ] All tests passing

---

_Iteration 01 - Decide complete_
_Next: Act - Implement the decided changes_
