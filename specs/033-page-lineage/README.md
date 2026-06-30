# SPEC-033 — Page Lineage: PDF Page Attribution in Data Hierarchy & Deeplinks

## Document Index

| File                                                       | Purpose                                    |
| ---------------------------------------------------------- | ------------------------------------------ |
| [00-why-first-principles.md](./00-why-first-principles.md) | 5-WHY analysis + First Principles          |
| [01-requirements.md](./01-requirements.md)                 | Functional + non-functional requirements   |
| [02-data-model.md](./02-data-model.md)                     | Data model changes (Rust types + TS types) |
| [03-architecture.md](./03-architecture.md)                 | System architecture & data flow diagrams   |
| [04-ux-ui-spec.md](./04-ux-ui-spec.md)                     | UX/UI layout, navigation, component spec   |
| [05-api-contract.md](./05-api-contract.md)                 | API contract changes (endpoints + DTO)     |
| [06-implementation-plan.md](./06-implementation-plan.md)   | Phased implementation with DRY/SOLID map   |

## Status

- Author: EdgeQuake Engineering
- Date: 2026-06-30
- State: DRAFT
- Implements: SPEC-033

## Executive Summary

PDF documents ingested through EdgeQuake already carry `page_start` / `page_end`
attribution on every chunk (SPEC-032 W-09). This spec closes the loop by surfacing
that page data end-to-end:

1. **Data Hierarchy**: group chunks under "Page N" nodes when page info is available.
2. **Deeplinks**: every chunk badge in the hierarchy tree navigates the PDF viewer to
   the correct page without a full page reload.
3. **Query Citations**: group citation passages by page number; each passage shows a
   "Go to page" badge that deeplinks to the document viewer at the right page.
4. **PDF Viewer control**: upgrade `PDFViewer` from uncontrolled (`initialPage`) to
   controlled (`currentPage` + `onPageChange`) so hierarchy-tree clicks drive it.
