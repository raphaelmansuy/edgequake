# OODA-22 Observe: Deep Analysis of Lowest-Scoring PDFs

## Date: 2025-02-03

## Target Documents

| PDF                   | Text% | Struct% | Overall% | Primary Issue  |
| --------------------- | ----- | ------- | -------- | -------------- |
| 01_2512.25075v1       | 72.2% | 88.7%   | 80.5%    | Text loss      |
| one_tool_2512.20957v2 | 73.7% | 78.1%   | 75.9%    | Both           |
| AlphaEvolve           | 85.6% | 74.3%   | 79.9%    | Structure loss |

## AlphaEvolve Analysis (74.3% Structural Fidelity)

### Comparison: Gold vs Extracted

**Gold Standard (Correct):**

```markdown
# AlphaEvolve: A coding agent for scientific and algorithmic discovery

**Google DeepMind**

**Authors:** Alexander Novikov, Ngân Vũ, Marvin Eisenberger...

### Abstract

In this white paper, we present AlphaEvolve, an evolutionary coding agent...
```

**Extracted (Issues):**

```markdown
# _Alpha Evolve_: A coding agent for scientific and algorithmic discovery

## 1. Introduction

**See , Swarat Chaudhuri , George Holland...**

Google Deep Mind

**In this white paper, we present\*\*\***Alpha Evolve**\***, an evolutionary...\*\*
```

### Root Cause Identification

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   STRUCTURAL FIDELITY LOSS ANALYSIS                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. SECTION ORDERING ERROR                                               │
│     • "1. Introduction" appears BEFORE abstract                         │
│     • Authors split and misplaced                                       │
│     • Root cause: Multi-column reading order confusion                  │
│                                                                          │
│  2. TEXT FRAGMENTATION                                                   │
│     • Sentences broken mid-word by line breaks                          │
│     • "substantially enhances** **capabilities" (broken bold)           │
│     • Root cause: Block merging not respecting sentence boundaries      │
│                                                                          │
│  3. BOLD/ITALIC ARTIFACTS                                               │
│     • "*Alpha Evolve*" instead of "AlphaEvolve"                        │
│     • Extra asterisks breaking words                                    │
│     • Root cause: Font style detection misinterpreting spacing          │
│                                                                          │
│  4. CITATION NUMBERS INLINE                                              │
│     • "[ 32 , 76 ]" with extra spaces                                   │
│     • "[83 ]" instead of "[83]"                                         │
│     • Root cause: Text grouping spacing issues                          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## 01_2512.25075v1 Analysis (72.2% Text Preservation)

### Gold File Quality Issue

The gold file starts with arXiv metadata fragments:

```
5
2
0
2

c
e
D
1
3
```

This is the arXiv identifier "2512.25075v1" displayed vertically in the PDF margin.
The gold file includes this garbage, which INFLATES the expected word count.

**Impact:** Low text preservation score may be partially due to POOR GOLD QUALITY,
not just extraction issues. The gold file counts vertical margin text as "expected" content.

### First Principles Analysis

If gold contains garbage characters that shouldn't be there, our TPS metric is biased.
We should:

1. Clean up gold files OR
2. Pre-filter common arXiv margin patterns from comparison

## one_tool_2512.20957v2 Analysis (75.9% Overall)

Similar patterns to AlphaEvolve:

- Multi-column reading order issues
- Author/affiliation block confusion
- Abstract fragmentation

## Key Findings

### Issue 1: Two-Column Reading Order (High Impact)

The current implementation sometimes outputs:

```
[LEFT_BLOCK_1]
[RIGHT_BLOCK_1]  <- Should come after ALL left blocks
[LEFT_BLOCK_2]
[RIGHT_BLOCK_2]
```

Instead of correct:

```
[LEFT_BLOCK_1]
[LEFT_BLOCK_2]
[RIGHT_BLOCK_1]
[RIGHT_BLOCK_2]
```

**Evidence from extraction logs:**

```
BMP-AFTER-MERGE PAGE1 (first 20 of 26):
     [0] X=55 Y=0 'One Tool Is Enough...'
ABS  [1] X=150 Y=84 'Abstract'
     [2] X=108 Y=50 'Authors...'
     [7] X=55 Y=430 '1. Introduction'
>>>  [10] X=307 Y=171 'Figure 1...'  <- RIGHT COLUMN CONTENT
```

The `>>>` marker shows right column content appearing mid-sequence.

### Issue 2: Block Merging Destroys Sentence Boundaries

```
BEFORE: 68 blocks, 2 columns
AFTER:  26 blocks
```

62% reduction. Some merges break mid-sentence.

### Issue 3: Gold File Quality

Some gold files contain extraction artifacts:

- Vertical arXiv IDs in margins
- Figure annotations mixed with text
- Table of contents duplicated

## Files to Investigate

| File                         | Issue                  |
| ---------------------------- | ---------------------- |
| backend/text_grouping.rs     | Block merging logic    |
| backend/extraction_engine.rs | Reading order assembly |
| backend/layout_processing.rs | Section ordering       |

## Proposed Fixes for OODA-23

1. **Reading Order Fix**: Ensure ALL left column blocks before ANY right column
2. **Sentence-Aware Merging**: Don't merge blocks that end mid-word
3. **Gold File Cleanup**: Remove arXiv margin artifacts from gold standards
4. **Citation Spacing**: Fix spacing around brackets in citations

## Expected Impact

| Fix            | Text       | Structure  | Overall |
| -------------- | ---------- | ---------- | ------- |
| Reading order  | +0%        | +5-7%      | +3%     |
| Sentence merge | +3-4%      | +2-3%      | +3%     |
| Gold cleanup   | +5-10%     | +0%        | +3%     |
| **Total**      | **+8-14%** | **+7-10%** | **+9%** |

With these fixes, we could reach ~90% overall quality.
