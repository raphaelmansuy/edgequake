# OBSERVE.md - Iteration 001

**Directory:** `crates/edgequake-pdf/src/backend`

## Test Results

- `cargo test -p edgequake-pdf`: **102/102 tests passed** ✅

## Evaluation Output

Ran `cargo run -p edgequake-pdf --example real_dataset_eval -- --write`

| Document              | Chars   | Camel Join | Hyphen Break | Double Space | ArXiv Header |
| --------------------- | ------- | ---------- | ------------ | ------------ | ------------ |
| 2900_Goyal_et_al      | 32,193  | 0          | 17           | 32           | 0            |
| AlphaEvolve           | 105,301 | 54         | 48           | 248          | 27           |
| agent_2510.09244v1    | 96,190  | 67         | 47           | 413          | 11           |
| ccn_2512.21804v1      | 26,462  | 0          | 9            | 0            | 0            |
| one_tool_2512.20957v2 | 52,020  | 49         | 22           | 828          | 25           |

## Validator SKILL Metrics

### Simple test-data:

- Table Accuracy: 100.0%
- Style Accuracy: 84.3%
- Robustness: 100.0%
- Performance: 90.0%
- **Composite Score: 92.7/100**

### Real Dataset (with gold files):

- Drifts found: 628
- By Category:
  - content: 468 (74.5%)
  - list: 59 (9.4%)
  - style: 54 (8.6%)
  - heading: 27 (4.3%)
  - table: 20 (3.2%)
- By Severity:
  - CRITICAL: 73
  - MAJOR: 285
  - MINOR: 270

## Key Issues Observed

### Issue 1: Inverted Reading Order (CRITICAL)

The generated `.mdf.gen` files show content in REVERSE order compared to the gold files:

**Gold (correct):**

```
### Abstract
...
## 1. Introduction
...
```

**Generated (wrong):**

```
1Link of code
tasks like Term Typing...
(inverted paragraph order)
```

### Issue 2: Ligature/Encoding Issues (MAJOR)

Words with ligatures are being incorrectly split or have missing characters:

- "specifc" instead of "specific" (missing 'i' - 'fi' ligature issue)
- "defnition" instead of "definition"
- "fne-tuning" instead of "fine-tuning"

### Issue 3: Word Concatenation (MAJOR)

Spaces missing between words:

- "silpnlp" instead of "silp_nlp"
- "fromgrok.coandgrok.com" instead of "from grok.co and grok.com"

### Issue 4: Running Headers Not Filtered (MINOR)

Running headers like "Goyal et al.|Open Conf Proc 6 (2025) ..." appear throughout the document.

## Root Cause Hypothesis

1. **Inverted Order**: The `sota_backend.rs` at line 2379 sorts blocks by `b.bbox.y2` descending, but this sorting happens AFTER the two-column processing which already ordered content correctly. The final sort destroys the column reading order.

2. **Ligature Issues**: The font encoding in `encodings::WIN_ANSI_ENCODING` doesn't properly handle ligature glyphs (fi, fl, ff, ffi, ffl) which are common in PDF fonts.

3. **Word Concatenation**: The `merge_line()` function's space detection threshold (avg_char_width \* 1.1) may be too large for fonts with tight kerning.

## Next Steps (Orient)

Focus on the backend directory to fix:

1. Remove or fix the final block sort that destroys reading order
2. Add ligature glyph support to the encoding tables
3. Improve space detection in merge_line()
