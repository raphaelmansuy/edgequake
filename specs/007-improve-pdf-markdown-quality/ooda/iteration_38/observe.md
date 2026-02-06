# OODA-38 Observe

## Current Quality State (after IT37)

- LightRAG PDF: 58993 bytes, 239 blocks across 16 pages
- 457 tests passing, 0 clippy warnings

## Issues Identified

### 1. Section Number Standalone ("3.2" alone on a line)

- "3.2" rendered on line 62, "DUAL-LEVEL RETRIEVAL PARADIGM" on line 64
- SectionNumberMergeProcessor only merged titles to the RIGHT (Mode A)
- In this PDF, titles are BELOW section numbers (Mode B)
- Also affects: 7.3.2, 7.3.3 in the appendix

### 2. Garbled Diagram Text (residual from IT37)

- "AgricultureEnvironmentalProductionImpact" (40 chars) — NOT caught by >40 threshold
- "OriginalRelationsTextincludes" (30 chars) — too short for IT37's checks
- Outer guard `trimmed.len() > 50` prevents checking shorter lines

### 3. Section Title Detection (ALL CAPS not handled)

- `looks_like_section_title("DUAL-LEVEL RETRIEVAL PARADIGM")` returned false
- ALL-CAPS text was classified as "person name" by the heuristic
- The keyword list didn't include "retrieval" or "paradigm"

## Quality Metrics Before

- Section number merging: ~60% (many standalone numbers)
- Garbled text: ~85% filtered (5 blocks in IT37, residual remains)
- Headers overall: ~80/100
