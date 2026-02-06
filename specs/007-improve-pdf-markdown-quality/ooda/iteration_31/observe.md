# IT31 — Observe: Dead Code Audit

## Mission Re-Read

Re-read `specs/007-improve-pdf-markdown-quality.md` at session start. Key directive:

> "Ensure to remove lopdf legacy code in order to improve the quality of the code: dead code is not good for maintenance: only keep one extraction, conversion pipeline for clarity."

## Current State

- IT30 committed as `5d17d7bc` — header over-promotion fix
- 571 lib tests passing (pre-removal)
- Two extraction backends coexist: PdfiumBackend (production) + ExtractionEngine (lopdf, deprecated)
- lopdf modules gated behind `#[cfg(feature = "lopdf")]` feature flag

## Dead Code Inventory

### lopdf Backend Modules (11,403 lines)

| File                  | Lines | Purpose                                             |
| --------------------- | ----- | --------------------------------------------------- |
| extraction_engine.rs  | 1,359 | Legacy lopdf-based extraction backend               |
| content_parser.rs     | 712   | PDF content stream parser                           |
| elements.rs           | 167   | Shared (kept — used by pdfium)                      |
| element_processing.rs | 389   | Content stream element processing                   |
| font_handling.rs      | 616   | Font name pattern matching                          |
| glyph_list.rs         | 391   | Adobe glyph name → Unicode mapping                  |
| encodings.rs          | 1,371 | PDF encoding/ToUnicode parsing                      |
| column_detection.rs   | 942   | lopdf column detection (superseded by geometric.rs) |
| spatial.rs            | 339   | Shared (kept — used by pdfium)                      |
| text_grouping.rs      | 1,542 | Character → word → line grouping                    |
| truetype_cmap.rs      | 201   | TrueType cmap table parsing                         |
| block_builder.rs      | 437   | Block construction from text groups                 |
| lattice.rs            | 1,711 | Table lattice detection                             |

### lopdf Image Pipeline (789 lines)

| File                          | Lines | Purpose                                 |
| ----------------------------- | ----- | --------------------------------------- |
| image_extraction.rs           | 482   | Image extraction via lopdf API          |
| processors/image_processor.rs | 307   | Pipeline processor for image extraction |

### Debug Binaries (1,410 lines)

All 10 debug binaries in `src/bin/` depend on lopdf for raw PDF inspection.

### Examples (7 files)

7 example files in `examples/` depend on lopdf for debugging.

## Total Dead Code: ~13,602 lines
