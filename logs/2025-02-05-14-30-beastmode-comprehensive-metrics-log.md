# Task Log: 2025-02-05 Comprehensive Quality Metrics

## Actions
- Analyzed current word-set F1 limitations (ignores order, duplicates, formatting)
- Researched ROUGE, BLEU, Levenshtein metrics from Wikipedia
- Created multi-dimensional evaluation script (`scripts/eval_comprehensive.py`)
- Ran comprehensive evaluation on all 7 gold standard files
- Updated mission spec with new Quality Metrics section
- Created OODA iteration 03 documentation (observe/orient/decide/act)

## Decisions
- Replace word-set F1 with Quality Score: `0.4×ROUGE-L + 0.3×Word_F1 + 0.15×Structure + 0.1×Format + 0.05×BLEU-4`
- Use ROUGE-L (LCS-based) as primary order metric (40% weight)
- Keep Word F1 for content accuracy but with multiset (not set)
- Add structural metrics (headings, paragraphs, lines)
- Add formatting metrics (bold, italic, lists)

## Next Steps
- Revert line_tolerance from 5pt to 3pt (caused regression)
- Implement smart sort key with vertical overlap detection
- Add column detection algorithm from pymupdf4llm
- Target: Improve ROUGE-L from 0.491 to 0.90+

## Lessons/Insights
- **Critical Discovery**: Old F1=0.877 was hiding real Quality=0.573 (43% overestimated)
- SET-based F1 is dangerous for order-sensitive text - use ROUGE-L instead
- Content extraction is working (F1=0.914), but ORDER is broken (ROUGE-L=0.491)
- True gap is 0.377 to target, not 0.073 as previously thought
