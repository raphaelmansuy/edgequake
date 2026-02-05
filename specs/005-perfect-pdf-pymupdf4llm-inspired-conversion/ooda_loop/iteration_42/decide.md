# OODA-42: Decide - Action Plan

## Date: 2026-02-05

## Decision Summary

Based on the analysis, here are the key decisions:

---

## Decision 1: Make pdfium the Default Backend

**Choice:** Change `default = ["lopdf"]` to `default = ["pdfium"]` in Cargo.toml

**Rationale:**

- Evaluation already uses pdfium and shows 0.786 quality
- lopdf font detection is unreliable (name matching vs flags)
- Mission spec explicitly says "Eliminate other backends such as lopdf"
- pdfium provides accurate character positions for reading order

**Risk Mitigation:**

- Keep lopdf as optional feature for environments without libpdfium
- Document libpdfium setup requirements
- Add CI check for both features

---

## Decision 2: Deprecate lopdf Modules, Don't Remove Yet

**Choice:** Add `#[deprecated]` warnings, schedule removal for OODA-70+

**Rationale:**

- Immediate removal would break existing deployments
- Gives users time to set up libpdfium
- Allows gradual migration with clear warnings
- Reduces risk of breaking changes

**Modules to Deprecate:**

```
backend/extraction_engine.rs  (1302 lines)
backend/text_grouping.rs      (1492 lines)
backend/content_parser.rs     (664 lines)
backend/font_handling.rs      (616 lines)
backend/element_processing.rs (389 lines)
backend/block_builder.rs      (398 lines)
backend/encodings.rs          (1317 lines)
backend/glyph_list.rs         (391 lines)
```

---

## Decision 3: Prioritize Quality Over Cleanup

**Choice:** First improve quality to 0.95, then do major refactoring

**Rationale:**

- Quality is the measurable goal (0.786 → 0.95)
- Refactoring without quality measurement leads to regressions
- Smaller, quality-focused changes are easier to validate

**Sequence:**

1. OODA 42-45: Make pdfium default, minimal changes
2. OODA 46-55: Quality improvements (Structure, ROUGE-L)
3. OODA 56-65: Code cleanup after quality target met

---

## Decision 4: pymupdf_grouper.rs Refactoring

**Choice:** Split 1362-line file into focused modules

**Rationale:**

- Single Responsibility Principle (SRP)
- Easier testing and debugging
- Better maintainability

**Proposed Split:**

```
layout/
├── grouper/
│   ├── mod.rs           - Public API and TextGrouper
│   ├── char_to_span.rs  - RawChar → Span conversion
│   ├── span_to_line.rs  - Span → Line grouping
│   ├── line_to_block.rs - Line → Block grouping
│   ├── classification.rs - BlockType classification
│   └── params.rs        - GroupingParams config
├── pymupdf_renderer.rs  - Keep as-is
└── pymupdf_structs.rs   - Keep as-is
```

---

## Implementation Order

### OODA-42: Baseline (This iteration)

- [x] Document architecture (Observe)
- [x] Gap analysis (Orient)
- [x] Action plan (Decide)
- [ ] Make minimal changes, run tests (Act)

### OODA-43: Make pdfium Default

- Update Cargo.toml: `default = ["pdfium"]`
- Update extractor.rs: Prefer pdfium when both available
- Run all tests, verify quality unchanged

### OODA-44: Add Deprecation Warnings

- Add `#[deprecated(since = "0.5.0", note = "Use pdfium feature")]`
- Update documentation
- Add migration guide

### OODA-45: Fix clippy Warnings

- Run `cargo clippy --all-features`
- Fix all warnings in edgequake-pdf
- Commit clean code

### OODA-46-55: Quality Improvements

Focus on Structure score (0.417 → 0.80):

- Heading level detection
- Paragraph boundary detection
- Line count matching

### OODA-56-65: Code Cleanup

- Split pymupdf_grouper.rs
- Remove duplicate code
- Add WHY comments and ASCII diagrams

---

## Acceptance Criteria for OODA-42

1. ✅ Architecture documented with ASCII diagrams
2. ✅ Gap analysis completed
3. ✅ Action plan defined
4. [ ] All tests pass
5. [ ] No quality regression (still 0.786+)
6. [ ] Files committed with "OODA-42:" prefix

---

## Files to Modify in Act Phase

| File             | Change                                |
| ---------------- | ------------------------------------- |
| `Cargo.toml`     | Document default feature rationale    |
| `backend/mod.rs` | Add deprecation notices               |
| `extractor.rs`   | Add WHY comment for backend selection |
| Mission spec     | Add OODA-42 to changelog              |

---

## Commit Message Template

```
OODA-42: Document pipeline architecture and establish baseline

- Create iteration_42/{observe,orient,decide,act}.md
- Document LEGACY (lopdf) vs NEW (pdfium) pipelines
- Identify 8,379 lines for deprecation in lopdf modules
- Confirm eval uses pdfium (quality = 0.786)
- Plan for making pdfium default in OODA-43
```
