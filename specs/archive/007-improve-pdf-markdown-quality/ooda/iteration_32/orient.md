# IT32 — Orient: Analysis

## Gap Analysis

The biggest quality gaps versus PyMuPDF4LLM are:

1. **Table detection (CRITICAL)**: Academic paper tables come out as jumbled text. The TableDetectionProcessor skips multi-column pages entirely (OODA-34 fix), which is correct for reading order but means tables within papers are never detected.

2. **Image/Figure extraction (CRITICAL)**: Figure labels and diagram text get extracted as regular text, producing garbled output. Need image extraction to assets/ folder.

3. **Debug logging pollution (LOW)**: Excessive tracing in hot paths violates SRP.

4. **Bug: extract_with_progress missing merge (HIGH)**: Blocks not merged = fragmented output for API callers using progress callbacks.

## Priority Ranking

1. Fix merge_same_line_blocks bug (quick, high-impact)
2. Remove debug logging (quick, improves readability)
3. Table detection for academic papers (complex, highest quality impact)
4. Image extraction (complex, spec requirement)
