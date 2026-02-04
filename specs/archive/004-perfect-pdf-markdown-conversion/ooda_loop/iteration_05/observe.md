# OODA-05: Observe

## Issue: Missing Title in hotmess PDF

### Markitdown Baseline

Using `mcp_markitdown_convert_to_markdown` on `hotmess_2601.23045v1.pdf`, the correct extraction shows:

```markdown
Published as a conference paper at ICLR 2026

THE HOT MESS OF AI: HOW DOES MISALIGNMENT
SCALE WITH MODEL INTELLIGENCE AND
TASK COMPLEXITY?

Aryo Pradipta Gema1,3
Alexander Hägele∗1,2
Jascha Sohl-Dickstein∗5
...
```

### Our Current Output

```markdown
## Page 1

Alexander Hagele ¨ ∗1,2 Aryo Pradipta Gema 1,3 Henry Sleight 4 Ethan Perez 5

Jascha Sohl-Dickstein ∗5 1Anthropic Fellows ProgramEPFLUniversity of Edinburgh...
```

**Missing elements:**

1. "Published as a conference paper at ICLR 2026" (header)
2. "THE HOT MESS OF AI: HOW DOES MISALIGNMENT SCALE WITH MODEL INTELLIGENCE AND TASK COMPLEXITY?" (title)

### Raw PDF Analysis

Using `pdftotext` tools:

```bash
# Raw mode (no layout) - correctly extracts title
$ pdftotext -raw -f 1 -l 1 hotmess.pdf -
Published as a conference paper at ICLR 2026
THE HOT MESS OF AI: HOW DOES MISALIGNMENT...

# Layout mode - shows spaced letters
$ pdftotext -layout -f 1 -l 1 hotmess.pdf -
T HE H OT M ESS OF AI: H OW D OES M ISALIGNMENT
S CALE W ITH M ODEL I NTELLIGENCE AND
TASK C OMPLEXITY ?
```

**Finding:** The title is embedded in the PDF with letter-spacing for visual emphasis. Some letters are grouped (`HE`, `OT`, `ESS`) while others are single with spaces between.

### Extraction Engine Analysis

From debug logs:

```
Page 1 - filtered Y range 74.8 to 743.7 (span=668.9), page_height=842.0, flipped=false
Page 1 pre-process: 41 raw text elements
Page 1 has 1 fonts
```

**Hypothesis:** The title text elements exist but are either:

1. Being filtered out due to Y coordinate bounds
2. Being sorted incorrectly after Y normalization
3. Not being extracted due to font encoding issues (only 1 font detected on page 1)

### Font Investigation

Page 1 has only 1 font detected, which is suspicious. Academic papers typically have:

- Title font (larger, bold)
- Author font (regular)
- Affiliation font (smaller)
- Body text font

If only 1 font is detected, some text elements may be using embedded subsets or Type3 fonts that aren't being parsed.

### Content Stream Investigation Needed

Need to examine the raw PDF content stream for page 1 to see:

1. What text operators are used (Tj, TJ, BT/ET)
2. What fonts are referenced
3. What Y coordinates the title text has
