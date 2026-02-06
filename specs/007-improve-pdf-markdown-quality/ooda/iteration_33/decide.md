# IT33 — Decide

## Actions

1. **Improve `table_like_score`** — Add percentage-rich detection (+3), numeric-line detection (+2), short-multiline bonus (+1)
2. **Add `is_percentage_value()` helper** — Pattern: digits with optional decimal + `%`
3. **Add `is_numeric_or_pct()` helper** — Combines float parse with percentage check
4. **Fix `parse_numeric_suffix`** — Strip `%` and `,` before float parse via `strip_numeric_decorators()`
5. **Add `try_column_reconstruction()`** — New function that detects column-oriented blocks and uses linearized grid parsing
6. **Add `parse_linearized_grid()`** — Parses `[label, val, val, ..., label, val, ...]` into table rows
7. **Update `scan_for_table()`** — Try column reconstruction first, fall back to row-oriented parsing
8. **Fix clippy warnings** — Replace consecutive `str::replace` with single-pass `strip_numeric_decorators`
9. **Add 9 unit tests** covering all new functions and edge cases

## Priority

CRITICAL — Tables are at 50/100, target is 80/100. This iteration focuses on academic paper tables which are the hardest category.

## Risk Assessment

- LOW: Scoring changes are additive (won't reduce scores for existing tables)
- LOW: Column reconstruction is a new code path tried before existing row parsing (no regression)
- MEDIUM: Linearized grid parser assumes regular [label, N values] pattern — may fail for irregular tables
