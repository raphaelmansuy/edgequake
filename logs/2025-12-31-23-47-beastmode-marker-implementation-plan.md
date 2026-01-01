# Task Log: Marker Architecture Implementation Plan

**Date:** 2025-12-31-23-47  
**Mode:** beastmode  
**Task:** Write full implementation plan for Marker-style architecture in EdgeQuake-PDF

---

## Actions

- Analyzed current EdgeQuake-PDF architecture (lib.rs, extractor.rs, config.rs)
- Reviewed LLMProvider trait for integration compatibility
- Created comprehensive 10-week implementation plan with 6 phases
- Designed block-based schema matching Marker's structure
- Defined Provider/Builder/Processor/Renderer pattern
- Documented XY-cut column detection algorithm
- Specified LLM service integration for enhancement mode
- Detailed vision mode with pdfium-render integration
- Designed CLI interface with all options

## Decisions

- Adopted Marker's modular architecture (Provider → Builder → Processor → Renderer)
- Chose XY-cut algorithm for column detection (proven, simple to implement)
- Made LLM enhancement optional via `--use_llm` flag (cost-conscious)
- Implemented hybrid mode as default for vision fallback
- Used pdfium-render for page rendering (cross-platform, well-maintained)
- Kept backward compatibility with existing `PdfExtractor` API

## Next Steps

1. Implement Phase 1: Block-based schema in `src/schema/`
2. Create `BlockType` enum with 18+ block types
3. Implement `BoundingBox` with intersection/IoU methods
4. Define `Document` and `Page` structures
5. Write unit tests for geometry operations

## Lessons/Insights

- Marker's architecture separates concerns cleanly: extraction → building → processing → rendering
- XY-cut algorithm handles multi-column layouts without ML models
- Vision mode should be optional feature gate to avoid heavy dependencies
- Quality assessment heuristics can trigger automatic vision fallback
- Position-aware text extraction is critical for layout detection
