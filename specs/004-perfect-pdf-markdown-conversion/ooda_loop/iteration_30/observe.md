# OODA-30 OBSERVE: ToUnicode CMap bfrange Parsing Bug

## Problem Statement

Apple-Sandbox-Guide-v1.0.pdf Page 2 extracts garbled text like `!"#$% '( )'*+%*+,` instead of "Table of Contents".

## Investigation

### Font Analysis (check_fonts.py)

Page 2 uses these fonts:

- `/F4.1 Cambria` - No Encoding, has FontFile2 (5739 bytes)
- `/F5.1 Calibri` - No Encoding, has FontFile2 (12108 bytes)
- `/F3.1 Calibri-Bold` - No Encoding, has FontFile2 (14930 bytes)

### Debug Tracing

Added debug output to `get_encoding()` function:

```
DEBUG get_encoding: Processing font 'NJKXJL+Calibri-Bold'
DEBUG get_encoding: Font 'NJKXJL+Calibri-Bold' has ToUnicode
DEBUG: Parsed 2 entries from ToUnicode CMap for 'NJKXJL+Calibri-Bold'
```

**KEY FINDING:** Calibri-Bold HAS a ToUnicode CMap, but we only parsed 2 entries!

### ToUnicode CMap Structure

Extracted the actual CMap content:

```
50 beginbfrange
<21><21><0054>
<22><23><0061>
<24><24><006c>
<25><25><0065>
...50 entries total...
endbfrange
```

### Root Cause Analysis

The bfrange parser uses `split_whitespace()`:

```rust
let parts: Vec<&str> = line.split_whitespace().collect();
if parts.len() >= 3 { ... }
```

But the CMap format `<21><21><0054>` has **NO SPACES** between hex codes!

```
DEBUG bfrange line: '<21><21><0054>' -> 1 parts  // WRONG - should be 3 parts!
```

This means **ALL 50 bfrange entries were being skipped** because `parts.len() >= 3` was never true.

## Impact

- All PDFs with concatenated bfrange entries have broken text extraction
- This includes many Microsoft Office-generated PDFs (Word, PowerPoint)
- Symptoms: garbled ASCII characters instead of readable text
