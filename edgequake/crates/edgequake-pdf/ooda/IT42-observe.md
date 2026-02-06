# OODA-IT42 Observe: Table Detection Producing Garbled Markdown

## Observation
Tables 1, 2, and 3 in LightRAG output are garbled with:
- Duplicated header rows
- Mixed raw text with markdown tables
- Inconsistent column counts
- Content from multiple pages interleaved incorrectly

## Sample Garbled Output
```
| Specific Retrieval Mode | Low-Level Queries | 85.4 | 69.1 | 90 | 79.8 |
| Title | Low-Level Queries | Answer Comprehensiveness (0-10) | Empowerment 3 |
| **Table 2: Multi-hop question comparison...** |
```

## Root Cause Analysis
1. `TableDetectionProcessor` uses heuristic column alignment detection
2. Academic PDFs have complex table layouts with merged cells, spanning headers
3. The processor creates tables that look correct structurally but have wrong content mapping
4. `TextTableReconstructionProcessor` then renders these broken tables as markdown

## WHY this matters (First Principles)
Tables encode structured relationships between data. If the extraction corrupts those relationships, the output is worse than no table at all - it actively misinforms readers.

## Decision Criteria
Plain text of table content preserves readability even if formatting is lost.
Garbled markdown tables actively mislead readers with wrong data in wrong columns.
**Plain text > Garbled tables**
