# Iteration 03: Orient

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Root Cause Analysis

### Why Monospace Detection is Suboptimal

The current approach relies on font name pattern matching:

```text
Font Name                  Detected?   Reality
───────────────────────────────────────────────
"CourierNew"               ✓           Monospace
"Consolas"                 ✓           Monospace
"Arial"                    ✗           Variable-width
"Menlo"                    ✗           Monospace (MISS!)
"Monaco"                   ✗           Monospace (MISS!)
"SFMono-Regular"           ✗           Monospace (MISS!)
"InputMono-Regular"        ✗           Monospace (MISS!)
```

**Accuracy**: ~70% (many monospace fonts don't have "mono" in name)

### PDFium Solution

PDFium provides `font_is_fixed_pitch()` which reads the PDF font descriptor:

```text
PDF Font Descriptor
├── FontName: "Menlo"
├── Flags: 0x00000022  (bit 1 = FixedPitch = TRUE)
├── FontWeight: 400
└── ...

PDFium API:
  char_obj.font_is_fixed_pitch() → bool (reads bit 1 of Flags)
```

**Accuracy**: ~99% (authoritative from PDF font metadata)

## Solution Strategy

```text
┌─────────────────────────────────────────────────────────────┐
│                  Data Flow (After Fix)                      │
│                                                             │
│  PDFium                  RawChar              Span          │
│  ───────                 ───────              ────          │
│  font_is_fixed_pitch() → is_monospace: bool → font_is_mono  │
│  (PDF descriptor)        (extracted)          space: Option │
│                                                             │
│  Span::is_monospace():                                      │
│  1. If font_is_monospace.is_some() → use it (99% accurate)  │
│  2. Else fallback to font name matching (70% accurate)      │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Steps

1. **Add `is_monospace: bool` to RawChar** (`backend/elements.rs`)
2. **Extract from PDFium** (`backend/pdfium.rs`)
   - `let is_monospace = char_obj.font_is_fixed_pitch();`
   - Track `last_is_monospace` for whitespace inheritance
3. **Add `font_is_monospace: Option<bool>` to Span** (`layout/pymupdf_structs.rs`)
4. **Update `Span::is_monospace()`** - prefer flag, fallback to name
5. **Update `can_append()`** - check monospace consistency (OODA-03 style)
6. **Add tests** - verify monospace span rejection
