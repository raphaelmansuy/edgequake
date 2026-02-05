# OODA-29 Orient: LLM Enhance Documentation Analysis

## Context

The LLM enhancement processor is the final stage of PDF extraction that uses AI to improve output quality. It handles:
- Table formatting (raw text → proper markdown tables)
- Math conversion (Unicode symbols → LaTeX)
- Image description (vision LLM)
- OCR error correction

## Risk Assessment

| Factor | Risk | Mitigation |
|--------|------|------------|
| Cost per document | High | Document when to enable each feature |
| Over-enhancement | Medium | Explain conservative defaults |
| Magic thresholds | Medium | Document heuristic origins |

## Key Decisions to Document

1. **Default: improve_text=false** - OCR error correction is aggressive and can modify correct text
2. **Threshold 0.3 for word characters** - Below this, text is likely garbage/symbols
3. **Pattern matching for OCR errors** - "nurnber", "0O", etc. are common OCR artifacts

## Alignment with Mission

Mission 006 goals:
- ✅ High signal WHY comments → Document enhancement strategy
- ✅ Clean code → Explain heuristic thresholds
- ✅ Test coverage → Add builder chain test

## Decision

1. Add WHY comment to LlmEnhanceProcessor
2. Add WHY to text_needs_improvement() explaining thresholds
3. Add test for processor with image OCR config
