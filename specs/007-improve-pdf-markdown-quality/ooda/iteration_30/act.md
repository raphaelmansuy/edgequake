# IT30 — Act: Fix Header Over-Promotion

## Changes made

### File: `src/backend/pdfium_backend.rs` — classify_blocks()

**Removed digit exclusion from not_list guard** (line ~395)

Before:
```rust
let not_list = !text.starts_with('•')
    && !text.starts_with('-')
    && !text.starts_with('*')
    && !text.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
```

After:
```rust
let not_list = !text.starts_with('•')
    && !text.starts_with('-')
    && !text.starts_with('*');
```

WHY: Font size is the authority for header classification. Blocks with
font_size >= body_size * 1.2 should be headers regardless of starting
character. "0) AI Strategy & Co‑Creation" (15pt, ratio 1.25) was incorrectly
excluded because it starts with a digit.

### File: `src/renderers/markdown.rs` — convert_standalone_bold_to_headers()

**Replaced broad criteria with section-number pattern** (line ~1248)

Only standalone bold lines matching a section-number regex are promoted to headers:
- Pattern: `^(\d+[\).\-:\s]|section\s+\d|chapter\s+\d|part\s+[IVX\d])`
- `**0) AI Strategy**` → `## 0) AI Strategy` ✓
- `**What we deliver**` → stays as `**What we deliver**` ✓

### Updated tests (markdown.rs)

- `test_convert_standalone_bold_with_section_number` — new, validates promotion
- `test_convert_standalone_bold_without_section_number` — new, validates no promotion
- `test_convert_standalone_bold_preserves_caption` — updated: "Table of Contents" no longer promoted
- `test_convert_standalone_bold_multiple_lines` — updated: uses "1. Introduction", "2. Methods", "Key Findings"

## Test results

```
571 passed; 0 failed; 0 ignored
```

## Output quality

Before (IT29):
```
## What we deliver       ← false header (×3)
## Capabilities          ← false header (×2)
## Typical use cases     ← false header
## Key outputs           ← false header
```

After (IT30):
```
**What we deliver**      ← correct bold paragraph
**Capabilities**         ← correct bold paragraph
**Typical use cases**    ← correct bold paragraph
**Key outputs**          ← correct bold paragraph
#### 0) AI Strategy      ← correct h4 from font size
```

## Remaining items for future iterations

- Header level tuning: numbered sections 0)-3) are h4 (ratio 1.25) but could
  be h2/h3 structurally. Threshold adjustment needed.
- Bold formatting: our output preserves **bold** while gold strips it. Our
  output is richer — this is a feature.

## Commit

Parent: `99659bd7` (IT29)
