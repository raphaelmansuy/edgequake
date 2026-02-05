# Iteration 05: Decide

**Mission**: Improve PDF to Markdown Extraction Pipeline  
**Mission File**: `specs/006-improve-pdf-markdown.md`

## Decision: Add Font Style Data Flow Diagram

### Rationale

The font style detection pipeline spans multiple files:

- `backend/pdfium.rs` - Extraction from PDFium
- `backend/elements.rs` - RawChar definition
- `layout/pymupdf_structs.rs` - Span definition and methods
- `layout/pymupdf_renderer.rs` - Markdown output

A comprehensive ASCII diagram at the top of `pymupdf_structs.rs` will:

1. Help new developers understand the system
2. Document the OODA improvements (OODA-02, OODA-03)
3. Explain why each field exists

### Implementation

Add module-level documentation to `pymupdf_structs.rs` with:

1. ASCII data flow diagram
2. WHY explanations for magic numbers
3. References to OODA iterations
