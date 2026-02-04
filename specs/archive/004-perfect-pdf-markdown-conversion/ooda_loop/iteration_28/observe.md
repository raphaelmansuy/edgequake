# OODA-28 Observe: Fast Quality Tests and New PDF Analysis

## Re-read Mission Status ✅

Re-read `specs/004-perfect-pdf-markdown-conversion.md` at iteration start.

## Key Mission Updates

The user has emphasized:

1. **Fast tests are critical** - Current tests crash VS Code due to excessive runtime
2. **Quick quality metric tests first** - Need instant feedback during development
3. **New PDF documents added** for testing in `zz_test_docs/`
4. **Use markitdown MCP** to compare quality and create gold standards

## Current Test Infrastructure

### Test Tiers (Working ✅)

| Tier          | File                       | Time  | Status              |
| ------------- | -------------------------- | ----- | ------------------- |
| Smoke         | `quick_smoke.rs`           | 0.09s | ✅ 4 tests pass     |
| Fast Quality  | `fast_quality.rs`          | 1.59s | ✅ 5 tests pass     |
| Comprehensive | `comprehensive_quality.rs` | 118s  | ⚠️ Too slow for dev |

### Current Quality Metrics

From OODA-27:

- Text Preservation: 81.4% (target: ≥98%)
- Structural Fidelity: 80.2% (target: ≥95%)
- Overall Quality: 80.8% (target: 95%+)

## New PDF Documents Analysis

### Documents in `zz_test_docs/`

| Document                        | Size   | Type            | Markitdown Quality           |
| ------------------------------- | ------ | --------------- | ---------------------------- |
| `Apple-Sandbox-Guide-v1.0.pdf`  | 354KB  | Technical guide | ✅ Excellent - clean text    |
| `stackplanner_2601.05890v1.pdf` | 1.1MB  | arXiv 2-column  | ⚠️ Fragmented chars at start |
| `lighrag_2410.05779v3.pdf`      | 1.1MB  | arXiv paper     | Untested                     |
| `Qwen.pdf`                      | 853KB  | Type3 fonts     | ✅ Fixed in OODA-01/02       |
| `agentfail_2601.22984v1.pdf`    | 1.7MB  | arXiv paper     | Has .md file                 |
| `hotmess_2601.23045v1.pdf`      | 5.8MB  | arXiv paper     | Has .md file                 |
| `kvzap_2601.07891v1.pdf`        | 5.1MB  | arXiv paper     | Large                        |
| French car manuals              | 6-10MB | Complex PDFs    | Too large for fast tests     |

### Markitdown vs EdgeQuake Comparison

#### Apple-Sandbox-Guide-v1.0.pdf (48 pages)

**Markitdown Output (excerpt):**

```
Apple's Sandbox Guide
v1.0
13-09-2011
© 2011, fG! - reverser@put.as

Apple's Sandbox Guide v1.0
Table	  of	  Contents
1	  –	  Introduction	  .....................................................................................................	  3
```

- ✅ Clean text extraction
- ✅ Table of contents preserved
- ⚠️ Extra tab characters in TOC

**EdgeQuake Output (excerpt):**

```
Apple

's Sandbox Guide

v1.0

13

2011

---

**!"#$% '( )'*+%*+,**
```

- ❌ Special characters corrupted (`!"#$%` instead of "Table")
- ❌ `5` characters appearing (dotted line encoding issue)
- ❌ Text fragmented across lines

**Root Cause Analysis:**

```
ASCII Diagram - Font Encoding Issue

PDF Internal:
┌─────────────────────────────────────────────┐
│ Font: /F1 (Custom encoding)                 │
│ ToUnicode: Missing or incomplete            │
│ Encoding: /WinAnsiEncoding or custom        │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│ Character Code → Glyph Name → Unicode       │
│                                             │
│ Code 0x54 → "T" glyph → U+0054 (correct)   │
│ Code 0x21 → "!" glyph → U+0021 (wrong!)    │
│                                             │
│ Some fonts map codes to DIFFERENT glyphs    │
│ Without ToUnicode, we get garbled text      │
└─────────────────────────────────────────────┘
```

This PDF likely uses a font with custom encoding where:

- The visual glyph for "Table" is rendered correctly in the PDF
- But the internal character codes map to different Unicode values
- Markitdown handles this correctly, we need to investigate why

#### stackplanner_2601.05890v1.pdf (arXiv)

**Markitdown Output:**

- First ~50 chars are fragmented (arXiv date header)
- Main content (title, abstract) extracted well
- Two-column layout handled reasonably

**EdgeQuake Output (from OODA-27):**

- Title and authors extracted correctly
- Abstract identified
- Better than markitdown on academic papers

## Test Speed Requirements

**Goal:** <5 seconds for development loop

```
Current test times:
├── quick_smoke.rs:    0.09s ✅
├── fast_quality.rs:   1.59s ✅
└── TOTAL:             1.68s ✅ (under 5s target)
```

**Needed Improvements:**

1. Add more diverse PDFs to fast_quality tests
2. Include font encoding test case
3. Add markitdown comparison metrics
4. Keep individual test <500ms

## Architecture Analysis - Font Decoding Pipeline

```
┌──────────────────────────────────────────────────────────┐
│                 PDF Font Decoding Flow                    │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  PDF Document                                            │
│       │                                                  │
│       ▼                                                  │
│  ┌──────────┐   ┌──────────────┐   ┌──────────────┐     │
│  │ Font Obj │──►│ Encoding Obj │──►│ ToUnicode    │     │
│  └──────────┘   └──────────────┘   │ CMap         │     │
│       │               │            └──────────────┘     │
│       │               │                   │             │
│       ▼               ▼                   ▼             │
│  ┌──────────────────────────────────────────────┐       │
│  │           Character Code Lookup               │       │
│  │  1. Try ToUnicode CMap (if exists)           │       │
│  │  2. Try Encoding differences                 │       │
│  │  3. Try Adobe Glyph List (PostScript)        │       │
│  │  4. Fall back to WinAnsi/MacRoman            │       │
│  └──────────────────────────────────────────────┘       │
│                        │                                 │
│                        ▼                                 │
│  ┌──────────────────────────────────────────────┐       │
│  │            Unicode Text Output                │       │
│  └──────────────────────────────────────────────┘       │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Files to Analyze

Key extraction files:

- `src/backend/extraction_engine.rs` - Main extraction logic
- `src/backend/encodings.rs` - Font encoding handling
- `src/backend/font_decode.rs` - Character decoding
- `src/backend/text_grouping.rs` - Text element grouping

## Summary

**Key Observations:**

1. ✅ Fast test infrastructure is working (<2s total)
2. ⚠️ Font encoding issues in Apple-Sandbox-Guide
3. ✅ EdgeQuake beats markitdown on arXiv two-column papers
4. ⚠️ Need to add new PDFs to fast_quality tests
5. ❌ Apple-Sandbox-Guide has severe text corruption

**Priority for OODA-28:**

1. Add fast quality tests for new smaller PDFs
2. Investigate font encoding issue in Apple-Sandbox-Guide
3. Create markitdown gold standards for comparison
4. Keep all tests under 5 seconds total
