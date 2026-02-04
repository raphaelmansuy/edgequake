# OODA-22 Decide: Clean Gold Files First

## Date: 2025-02-03

## Decision

Clean up the 01_2512.25075v1.gold.md file to remove arXiv margin artifacts.

## Rationale

1. **Lowest effort, highest immediate impact** on TPS score
2. **No code risk** - only affects test data, not production code
3. **More accurate future measurements** - removes bias from metrics
4. **Discovery-driven** - found actual gold file quality issue

## Specific Changes

### 01_2512.25075v1.gold.md

Remove the arXiv identifier margin text at the start:

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

]

V
C
.
s
c
[

1
v
5
7
0
5
2
.
2
1
5
2
:
v
i
X
r
a
```

This is the arXiv ID "2512.25075v1" displayed vertically in the PDF margin.

## Expected Impact

| Metric          | Before | After (Expected) |
| --------------- | ------ | ---------------- |
| 01_2512 TPS     | 72.2%  | 78-82%           |
| Overall TPS     | 81.3%  | 82-84%           |
| Overall Quality | 80.8%  | 82-83%           |

## Test Plan

1. Edit gold file to remove margin text
2. Run comprehensive tests
3. Verify TPS improvement for 01_2512.25075v1
4. Verify no regression on other PDFs

## Commit Message

```
OODA-22: Clean arXiv margin artifacts from gold file

WHY: The 01_2512.25075v1.gold.md file contained the arXiv
identifier displayed vertically in the PDF margin. This
garbage text artificially lowered TPS scores.

WHAT: Removed "5 2 0 2 c e D 1 3 ] V C . s c [ 1 v 5 7 0 5 2"
and related arXiv margin fragments from gold file.

EXPECTED: +6-10% TPS improvement for this document.
```
