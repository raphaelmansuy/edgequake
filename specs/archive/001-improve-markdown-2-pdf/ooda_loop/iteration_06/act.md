# OODA Iteration 06 – Act

**Date:** 2026-02-06
**Theme:** Bold-Only Header Detection for Academic Papers

## Changes Made

- **Modified:** `block_classifier.rs` — re-enabled pattern detection behind `is_all_bold()` gate.
- **Added:** `is_all_caps_header()` with keyword allowlist for academic section dividers.
- Bold detection: all spans bold AND text matches recognized pattern.
- Level mapping: Roman prefix → H2, numeric section → H2, subsection → H3, all-caps → H2.

## Commit

`f6ef3709` — Bold-only header detection for academic papers.

## Test Results

- **495 tests passing.** No regressions. 2 new unit tests added and green.

## Outcome

Academic paper headers (IEEE, NeurIPS) now correctly classified with zero impact on non-academic PDFs.

**Mission Re-read:** Confirmed.
