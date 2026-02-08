# OODA Iteration 05 – Decide

**Date:** 2026-02-06
**Theme:** Hyphenation Resolution Across Line Breaks

## Decisions

1. Create `layout/hyphenation.rs` with `resolve_hyphenation(text: &str) -> String`.
2. Integrate into `render_lines_inline()` after line joining.
3. Soft hyphen (U+00AD) + `\n` → remove both, join fragments.
4. ASCII hyphen + `\n` + lowercase → remove hyphen and newline, join.
5. ASCII hyphen + `\n` + uppercase/digit/marker → preserve as-is.

## Test Plan — 10 Cases

| # | Case              | Input              | Expected         |
|---|-------------------|---------------------|-------------------|
| 1 | Soft hyphen       | `compu\u{AD}\nter` | `computer`        |
| 2 | ASCII + lowercase | `compu-\nter`       | `computer`        |
| 3 | ASCII + uppercase | `New-\nYork`        | `New-\nYork`      |
| 4 | Hard mid-line     | `self-contained`    | `self-contained`  |
| 5 | ASCII + digit     | `pre-\n2024`        | `pre-\n2024`      |
| 6 | Multiple breaks   | `a-\nb c-\nd`       | `ab cd`           |
| 7 | Empty/no hyphens  | `hello world`       | `hello world`     |
