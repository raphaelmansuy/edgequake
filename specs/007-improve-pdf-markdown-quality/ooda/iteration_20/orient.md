# OODA Iteration 20 - Orient

## Root Cause Analysis

The column detection in `column_detection.rs` uses zone-based classification:

- Elements with `x < page_width * 0.45` → left zone
- Elements with `x > page_width * 0.48` → right zone
- If left >= 3 AND right >= 3 AND balance > 0.15 → TWO-COLUMN

For a table PDF (004_simple_table_2x3.pdf), the table cells distribute as:

- Left zone: Name(x=218), Alice(x=223), Bob(x=225), Charlie(x=218) + spanning title/description
- Right zone: Age(x=367), 25(x=372), 30(x=372), 35(x=372)

This passes the two-column test, but it's actually a table grid.

## Key Insight: Table vs Column Discriminator

```
┌─────────────────────────────────────────────────────────────┐
│  TWO-COLUMN TEXT              TABLE GRID                    │
├─────────────────────────────────────────────────────────────┤
│  Long text per element        Short text per element        │
│  (sentences, paragraphs)      (names, numbers, labels)      │
│  Avg > 30 chars/element       Avg < 15 chars/element        │
│                                                             │
│  Independent vertical flow    Precise Y-alignment           │
│  (each column reads top→bot)  (rows align across columns)   │
│  Few Y-aligned pairs          Many Y-aligned pairs (>60%)   │
│                                                             │
│  Blocks are tall/wide         Blocks are short/narrow       │
│  (paragraphs)                 (cells)                       │
└─────────────────────────────────────────────────────────────┘
```

## Two Conditions for Table Detection

1. **Short text**: Average text length per non-spanning element < 15 chars
2. **Y-alignment**: > 60% of right-zone elements have a Y-aligned partner in left zone

Both conditions must be true to classify as table (not columns).

## Impact Chain

```
Column detection returns table-as-columns
   → Page gets 2 columns
   → TableDetectionProcessor sees multi-column → SKIPS (OODA-34)
   → Table content rendered as plain text with wrong reading order
   → Table score: 0/100 for this test case
```

Fix:

```
Column detection detects table pattern → returns SINGLE column
   → Page gets 1 column
   → TableDetectionProcessor runs → detects table
   → Table rendered as markdown table
   → Table score: 80+/100
```
