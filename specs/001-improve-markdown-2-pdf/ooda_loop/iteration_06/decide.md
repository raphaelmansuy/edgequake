# OODA Iteration 06 – Decide

**Date:** 2026-02-06
**Theme:** Bold-Only Header Detection for Academic Papers

## Decision

1. Re-enable pattern detection in `block_classifier.rs`, gated behind `is_all_bold()`.
2. Add new function `is_all_caps_header()` for standalone keywords (REFERENCES, ACKNOWLEDGMENTS, ABSTRACT, etc.).
3. Header level mapping:
   - Roman numeral sections (`II. RELATED WORK`) → **H2**
   - Numeric top-level sections (`3. Evaluation`) → **H2**
   - Numeric subsections (`3.1 Setup`) → **H3**
   - All-caps standalone lines → **H2**
4. Gate: block must satisfy `is_all_bold() && pattern_match()` to promote.
5. Add 2 unit tests:
   - Bold Roman numeral line classified as H2.
   - Bold all-caps keyword classified as H2.

## Risks

- All-caps body lines that happen to be bold (mitigated: keyword allowlist).
- Numbered list items misclassified (mitigated: require bold gate).

**Mission Re-read:** Confirmed.
