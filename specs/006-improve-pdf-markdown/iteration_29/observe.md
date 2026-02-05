# OODA-29 Observe: LLM Enhance Module Documentation Gap

## Current State

The `llm_enhance.rs` module (604 lines) handles LLM-based content enhancement but has 0 WHY comments.

## File Analysis

- **Size**: 604 lines
- **WHY comments**: 0
- **Tests**: 5 (3 sync + 2 async)
- **Total lib tests**: 484

## Key Functions Lacking WHY Documentation

1. `text_needs_improvement()` - Heuristics for OCR error detection
2. `enhance_table()` - Why use LLM for table formatting?
3. `convert_math()` - Why LLM for math conversion?
4. `describe_image()` - Vision LLM integration

## Observations

1. The module handles several enhancement types but lacks explanation of when each is appropriate
2. The `text_needs_improvement()` function uses magic thresholds (0.3, 0.5) without explanation
3. Builder pattern is well-implemented but lacks WHY for defaults

## Recommendation

1. Add WHY comment to LlmEnhanceProcessor explaining the enhancement strategy
2. Add WHY to text_needs_improvement() explaining the heuristics
3. Add test for config builder chain
