# Task Log - Loop 012: Table Column Detection with Geometric Clustering

## Date

Friday, January 2, 2026 20:30:00 HKT

## Actions

1. Committed previous changes (heading detection work)
2. Read spec/27-improve-pdf.md - mission to improve PDF→Markdown via OODA loops
3. Ran baseline metrics: Composite 32.5/100, Table 2.4%, Style 31.5%
4. User feedback: "focus on tables" - pivoted from heading detection
5. Analyzed actual PDF tables vs gold/generated markdown
6. Identified root cause: whitespace-based column detection fails
7. Implemented geometric clustering (DBSCAN) for column detection
8. Added dbscan_1d() function to geometric.rs
9. Replaced detect_columns_by_whitespace() with detect_columns_by_clustering()
10. Tested: All 111 tests pass, columns detected improved (2 → 13 cols)
11. Validated: Composite still 32.5/100 (cell content extraction remains broken)

## Decisions

- **Pivoted from headings to tables**: User correctly identified tables (2.4% with 40% weight) as higher ROI than headings
- **Used geometric clustering**: Applied same first-principles DBSCAN approach that succeeded in Loop 004 (+14.6 style points)
- **Adaptive epsilon**: 10th percentile of inter-element distances (data-driven, no magic numbers)
- **Stopped heading detection work**: Was low-impact, gold files may not be perfect

## Next Steps

1. **Fix extract_text_in_rect()**: Current method has ±2pt tolerance and 5pt Y-binning which loses precision
2. **Implement proper cell boundary matching**: Use tighter tolerances and exact row/column grid alignment
3. **Handle merged cells**: Detect colspan/rowspan patterns in table structure
4. **Test with validator**: Target Table Accuracy 15-20% (+5-7 composite points)

## Lessons/Insights

- **Don't chase gold files blindly**: User insight to check actual PDFs was critical
- **Focus on ROI**: Table Accuracy (2.4%) vs Style (31.5%), both 40% weight → tables higher opportunity
- **Reuse successful patterns**: DBSCAN worked for column detection (Loop 004), applied to table columns (Loop 012)
- **Structural vs content**: Fixed column detection structure but content mapping still broken - two separate problems
- **Listen to user feedback**: "You must focus on table" redirected from unproductive heading work

## Metrics Summary

- **Before Loop 012**: Composite 32.5, Table 2.4%, Style 31.5%
- **After Loop 012**: Composite 32.5, Table 2.4%, Style 31.5% (no score change but structural improvement)
- **Column Detection**: 2 columns → 13 columns (structural fix, not reflected in scores yet)
- **Tests**: 111/111 passing
