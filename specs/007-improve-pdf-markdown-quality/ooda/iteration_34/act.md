# IT34 — Act

## Changes Made

### 1. Fixed 5 clippy warnings in edgequake-pdf

**table_detection.rs:581**: `.skip(1).next()` → `.clone().nth(1)`
**markdown.rs:1248-1249**: Added blank line before OODA-30 comment to fix doc list indentation
**markdown.rs:1567**: `&next_trimmed[2..]` → `strip_prefix("- ")`
**markdown.rs:1588**: `&prev_trimmed[..len()-1]` → `strip_suffix('-')`

### 2. Downgraded 35+ INFO logs to debug/trace

| File                   | Changes              |
| ---------------------- | -------------------- |
| column_detector.rs     | 6 → debug            |
| geometric.rs           | 2 → debug            |
| reading_order.rs       | 3 → debug, 1 → trace |
| table_detection.rs     | 10 → debug           |
| layout_processing.rs   | 8 → debug            |
| structure_detection.rs | 3 → debug            |
| markdown.rs            | 1 → trace            |

### Before/After output

**Before** (60+ INFO lines):

```
INFO COLUMN-DETECT: 14 items, page_width=612
INFO COLUMN-DETECT: filtered 14 items to 14
INFO COLUMN-DETECT: clusters too close
...58 more lines...
```

**After** (5 INFO lines):

```
INFO Using PdfiumBackend for PDF extraction
INFO Starting PDF extraction to Markdown
INFO Starting PDF extraction to Document IR
INFO PdfiumBackend: extracting PDF (1123301 bytes)
INFO PdfiumBackend: extracted 16 pages, 217 total blocks
```

## Test Results

- **449 lib tests pass** (0 failures)
- **clippy**: 0 warnings in edgequake-pdf (was 5)
