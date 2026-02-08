# OODA Iteration 06 – Observe

**Date:** 2026-02-06
**Theme:** Bold-Only Header Detection for Academic Papers

## Observations

- Current block classifier relies solely on font-size ratio to detect headers.
- Academic PDFs (IEEE, NeurIPS, ACL) use bold section headers at body font size.
- These headers are invisible to the size-ratio heuristic and render as body text.
- Pattern detection functions (Roman numerals, numeric sections) exist in the codebase but were DISABLED during an earlier refactor.
- Bold spans are already extracted per-block; the `is_all_bold()` check is available.
- Common academic header patterns observed:
  - `I. INTRODUCTION`, `II. RELATED WORK` (Roman numeral + all-caps)
  - `3.1 Evaluation` (numeric subsection)
  - `REFERENCES`, `ACKNOWLEDGMENTS` (standalone all-caps keywords)

## Key Constraint

Font-size alone is insufficient; bold weight combined with textual patterns is required.

**Mission Re-read:** Confirmed.
