# Task Log: PDF Improvement OODA Session

## Date: 2025-01-15 13:10

---

## Actions

- Fixed reading order bug in `reading_order.rs` (Y coordinate sorting direction)
- Fixed ligature handling in `sota_backend.rs` (added `get_ligature_expansion()` fallback)
- Fixed validator bug in `validate.py` (index out of range error)
- Removed debug logging from production code
- Ran clippy and fmt, fixed trailing whitespace issues
- Created OODA session documentation for both iterations

## Decisions

- Added ligature mappings for both PostScript Type 1 (0x02-0x06) and Windows/Adobe (0x1B-0x1F) byte positions
- Override ToUnicode CMap when it returns just 'f' for known ligature positions (handles corrupted CMaps)
- Focus on text extraction quality first, style/table improvements for future iterations

## Next Steps

- Future iteration: Improve font weight detection via `/FontDescriptor` parsing
- Future iteration: Improve table cell grouping algorithm
- Future iteration: Document-level font analysis for heading levels

## Lessons/Insights

- PDF fonts use different byte positions for ligatures depending on font type (PostScript vs Adobe)
- Some PDFs have inconsistent ToUnicode CMaps that contradict their /Differences arrays
- PDF Y=0 is at bottom of page, not top - ascending Y gives bottom-to-top order
