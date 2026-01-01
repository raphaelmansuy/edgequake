# Implementation Plan: SOTA Table Extraction

## Phase 1: Foundation (Graphical Extraction)

- [ ] **Task 1.1:** Modify `extract_text_elements` to also extract graphical lines (`m`, `l`, `re` operators).
  - Create `PdfLine` struct.
  - Update `extract_page` to collect lines.
- [ ] **Task 1.2:** Implement `Line` merging logic (combine collinear overlapping segments).

## Phase 2: Lattice Engine (Explicit Tables)

- [ ] **Task 2.1:** Implement `detect_lattice_tables(lines, text_elements)`.
  - Find intersections.
  - Construct grid.
  - Assign text to cells.
- [ ] **Task 2.2:** Convert Lattice tables to `BlockType::Table`.

## Phase 3: Stream Engine (Implicit Tables)

- [ ] **Task 3.1:** Implement `detect_stream_tables(text_elements)`.
  - Identify candidate regions (high density of short text spans).
  - Detect columns via X-projection gaps.
  - Detect rows via Y-overlap.
- [ ] **Task 3.2:** Validate candidates (distinguish from multi-column prose).

## Phase 4: Integration & Refinement

- [ ] **Task 4.1:** Integrate into `SotaBackend::extract_page`.
  - Run table detection _before_ column/line grouping.
  - Filter out table text from the main text flow.
- [ ] **Task 4.2:** Implement Markdown rendering for tables.
- [ ] **Task 4.3:** Tune heuristics (gap thresholds, density checks) using the `one_tool` PDF.

## Phase 5: Testing & OODA Loop

- [ ] **Task 5.1:** Run extraction on `one_tool_2512.20957v2.pdf`.
- [ ] **Task 5.2:** Verify table output in Markdown.
- [ ] **Task 5.3:** Iterate on thresholds if tables are broken or missed.
