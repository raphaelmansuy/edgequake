# OODA Iteration 03 - Observe

**Mission**: Improve PDF-to-Markdown Conversion Quality
**Mission File**: `specs/007-improve-pdf-markdown-quality.md` (RE-READ ✓)
**Date**: 2026-02-05

---

## 1. Focus Area: List Detection and Bullet Preservation

### 1.1 Mission Priority

From mission file:

> | Lists (nested) | 55/100 | 85/100 | High |

Current list detection score is 55/100, target is 85/100.

---

## 2. PyMuPDF4LLM Analysis

### 2.1 Bullet Character Set (utils.py lines 28-56)

PyMuPDF4LLM has a comprehensive BULLETS tuple:

```python
BULLETS = tuple(
    {
        chr(0x2A),   # * asterisk
        chr(0x2D),   # - hyphen
        chr(0x3E),   # > greater than
        chr(0x6F),   # o lowercase o
        chr(0xB6),   # ¶ pilcrow
        chr(0xB7),   # · middle dot
        chr(0x2010), # ‐ hyphen
        chr(0x2011), # ‑ non-breaking hyphen
        chr(0x2012), # ‒ figure dash
        chr(0x2013), # – en dash
        chr(0x2014), # — em dash
        chr(0x2015), # ― horizontal bar
        chr(0x2020), # † dagger
        chr(0x2021), # ‡ double dagger
        chr(0x2022), # • bullet
        chr(0x2212), # − minus sign
        chr(0x2219), # ∙ bullet operator
        chr(0xF0A7), # private use
        chr(0xF0B7), # private use
        REPLACEMENT_CHARACTER,
    }
    | set(map(chr, range(0x25A0, 0x2600)))  # Block Elements + Geometric Shapes!
)
```

**Key insight**: Range 0x25A0-0x2600 includes:

- ▪ BLACK SMALL SQUARE (0x25AA)
- ■ BLACK SQUARE (0x25A0)
- □ WHITE SQUARE (0x25A1)
- ◆ BLACK DIAMOND (0x25C6)
- ○ WHITE CIRCLE (0x25CB)
- ● BLACK CIRCLE (0x25CF)
- ► RIGHT-POINTING TRIANGLE (0x25BA)
- And 512 more geometric shapes!

### 2.2 Bullet Detection (utils.py lines 182-193)

```python
def startswith_bullet(text):
    if not text:
        return False
    if not text.startswith(BULLETS):
        return False
    if len(text) == 1:
        return True
    if text[1] == " ":  # bullet followed by space
        return True
    return False
```

### 2.3 Indentation Calculation (pymupdf_rag.py lines 733-739)

```python
if startswith_bullet(text):
    text = "- " + text[1:]  # Replace bullet with dash
    dist = span0["bbox"][0] - clip.x0  # Distance from left margin
    cwidth = (span0["bbox"][2] - span0["bbox"][0]) / len(span0["text"])
    if cwidth == 0.0:
        cwidth = span0["size"] * 0.5
    text = " " * int(round(dist / cwidth)) + text  # Add indent spaces
```

---

## 3. Our Current Implementation

### 3.1 Bullet Regex (structure_detection.rs line 513)

```rust
let bullet_regex = Regex::new(r"^[-–—*•◦▪]\s+").unwrap();
```

**Problem**: Only 7 bullet characters! Missing 500+ from PyMuPDF4LLM.

### 3.2 List Patterns

- Number regex: `^\d+[\.)]\s+`
- Number no space: `^\d+\.[A-Z]`
- Reference: `^\[\d{1,3}\]\s*`

### 3.3 Indentation Calculation (markdown.rs lines 247-262)

```rust
let level = if let Some(lvl) = block.metadata.get("level")... {
    lvl.max(0) as usize
} else if let Some(indent) = block.metadata.get("indent")... {
    let lvl = ((indent - 72.0).max(0.0) / 20.0).floor() as usize;
    lvl
} else {
    0
};
```

**Issue**: Uses 72pt margin assumption (US Letter), may not work for all PDFs.

---

## 4. Gap Analysis

| Aspect           | PyMuPDF4LLM          | Our Implementation |
| ---------------- | -------------------- | ------------------ |
| Bullet chars     | 530+ (BULLETS tuple) | 7 (regex only)     |
| Detection method | startswith_bullet()  | Regex pattern      |
| Output format    | Always "- "          | Preserves original |
| Indent calc      | Distance from clip   | Fixed 72pt margin  |
| Geometric shapes | Yes (0x25A0-0x2600)  | No                 |

---

## 5. Test Baseline

```
cargo test --lib
test result: ok. 498 passed
```
