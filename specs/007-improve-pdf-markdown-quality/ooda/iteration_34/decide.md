# IT34 — Decide

## Actions

1. Fix 5 clippy warnings:
   - table_detection.rs: `.skip(1).next()` → `.nth(1)`
   - markdown.rs: Fix doc comment indentation
   - markdown.rs: Use `strip_prefix("- ")` instead of manual `[2..]`
   - markdown.rs: Use `strip_suffix('-')` instead of manual `[..len()-1]`

2. Downgrade verbose INFO logs to debug/trace in:
   - column_detector.rs (6 logs → debug)
   - geometric.rs (2 logs → debug)
   - reading_order.rs (5 logs → debug/trace)
   - table_detection.rs (10 logs → debug)
   - layout_processing.rs (8 logs → debug)
   - structure_detection.rs (3 logs → debug)
   - markdown.rs renderer (1 log → trace)

3. Keep INFO level ONLY for:
   - Backend initialization ("Using PdfiumBackend...")
   - Extraction start/complete ("Starting PDF extraction...")
   - Progress reporting (progress.rs module)

## Risk: LOW

All changes are logging/style — no algorithmic changes.
