# Scratchpad: SOTA PDF Table Extraction (No ML)

## Current Status

- [ ] Research SOTA algorithmic approaches
- [ ] Analyze current `sota_backend.rs` capabilities
- [ ] Draft Specification
- [ ] Draft Implementation Plan
- [ ] Draft Test Plan
- [ ] Execute OODA Loop (Iteration 1)

## Notes

- Focus on "One Tool Is Enough" paper PDF as the primary test case.
- The goal is to extract tables correctly into Markdown.
- No Machine Learning allowed.
- Must use geometric analysis (lines, whitespace, alignment).

## OODA Loop Log

### Iteration 1

- **Observe**: Current code handles text columns but likely fails on complex tables.
- **Orient**: Need to understand how to detect table structures (rows/cols) from raw text elements.
- **Decide**: Research Tabula and Camelot algorithms.
- **Act**: Fetch web pages.
