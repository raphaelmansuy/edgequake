# Task Log: SOTA PDF-to-Markdown Research

**Date:** 2025-12-31-23-41  
**Mode:** beastmode  
**Task:** Research best PDF-to-markdown libraries comparable to pyzerox

---

## Actions

- Fetched and analyzed 7 major PDF-to-markdown libraries from GitHub
- Compared architectures: text extraction vs. vision-first vs. hybrid approaches
- Created comprehensive comparison document at `pdf_extraction_plan/sota_library_comparison.md`
- Identified Marker (datalab-to) as current SOTA with 95.67% accuracy benchmark

## Decisions

- Prioritized Marker's hybrid LLM architecture as best model for EdgeQuake-PDF improvements
- Identified Surya as the key underlying OCR/layout toolkit powering Marker
- Determined vision mode should be optional (expensive but accurate)
- Recommended modular architecture with Provider/Builder/Processor/Renderer pattern

## Next Steps

1. Update existing scratchpad with prioritized implementation plan
2. Implement block-based extraction following Marker's schema
3. Add layout detection using heuristics (Priority 1)
4. Implement optional LLM enhancement mode (Priority 2)
5. Consider porting Surya models to Rust via candle/ONNX

## Lessons/Insights

- Marker benchmarks at 95.67% accuracy, 25 pages/second - sets the target
- Hybrid approach (text extraction + optional LLM) offers best cost/accuracy tradeoff
- Vision-first (pyzerox) is excellent for complex layouts but expensive ($0.01-0.03/page)
- MinerU offers 109-language OCR with hybrid VLM backend (newly released v2.7.0)
- PyMuPDF4LLM is fastest for simple PDFs but lacks deep learning models

---

## Libraries Researched

| Library     | Stars | Key Insight                             |
| ----------- | ----- | --------------------------------------- |
| MinerU      | 51.3k | Hybrid VLM+pipeline, 109 language OCR   |
| Docling     | 48.5k | IBM, MIT license, MCP server for agents |
| Marker      | 30.7k | **Highest accuracy (95.67%)**, fastest  |
| Surya       | 19.1k | OCR toolkit behind Marker, 97% OCR acc  |
| Nougat      | 9.8k  | Academic papers only, limited language  |
| PyMuPDF4LLM | 1.2k  | Very fast, rule-based, no deep learning |
| PyZerox     | ~1k   | Vision-first, expensive but accurate    |
