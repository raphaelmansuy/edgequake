# OODA-17: Orient - Analysis and Solution Options

## Date: 2026-02-01

## Problem Analysis

### First Principles Assessment

The mission spec states:
> "Display progression of ingestion for both PDF and Markdown in Documents panel"

The 6-phase pipeline is already defined:
1. `Upload` - File upload
2. `PdfConversion` - PDF → Markdown (PDF only)
3. `Chunking` - Document chunking
4. `Extraction` - Entity/relationship extraction
5. `Embedding` - Vector embedding generation
6. `GraphStorage` - Graph/vector storage

**Gap**: Phases 3-6 are not tracked for PDF documents after conversion.

### Root Cause

```
┌────────────────────────────────────────────────────────────────────┐
│                     PDF Processing Flow                             │
├────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  pdf_upload.rs                                                      │
│      │                                                              │
│      ▼                                                              │
│  PipelineProgressCallback ──► Updates PdfConversion phase ✓        │
│      │                                                              │
│      ▼                                                              │
│  Spawns TextInsert task with source_type="pdf"                     │
│      │                                                              │
│      ▼                                                              │
│  process_text_insert() ──► Updates document status                 │
│      │                    but NOT PdfUploadProgress phases ✗       │
│      │                                                              │
│      ▼                                                              │
│  Chunking → Extraction → Embedding → GraphStorage                  │
│      (No phase progress updates!)                                   │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

## Solution Options

### Option A: Add Phase Tracking in processor.rs

**Approach**: Add conditional PDF phase tracking calls in `process_text_insert()` when `source_type == "pdf"`.

**Pros**:
- Minimal code changes
- Uses existing tracking infrastructure
- No architectural changes needed
- Maintains single responsibility (processor handles processing)

**Cons**:
- Adds conditional logic to processor
- PDF-specific code in generic processor

**Risk**: Low - Uses existing, tested methods.

### Option B: Create Unified Progress Callback

**Approach**: Create new `UnifiedProgressCallback` that handles all phases for all document types.

**Pros**:
- Single callback for all progress updates
- Cleaner separation of concerns

**Cons**:
- Significant refactoring required
- More complex callback chain
- Higher risk of introducing bugs

**Risk**: Medium - Requires more extensive changes.

### Option C: Event-Based Progress System

**Approach**: Implement event-driven progress updates via message bus.

**Pros**:
- Decoupled architecture
- Future-proof for scaling

**Cons**:
- Over-engineering for current needs
- Significant implementation effort

**Risk**: High - Major architectural change.

## Recommendation

**Option A: Add Phase Tracking in processor.rs**

Rationale:
1. Uses existing `pipeline_state.start_pdf_phase()` and `complete_pdf_phase()` methods
2. Minimal code changes with immediate impact
3. Follows KISS principle
4. Can be done without breaking existing functionality
5. Aligns with mission spec for unified status tracking

## Implementation Plan

1. In `process_text_insert()`:
   - Detect if source is PDF via `source_type == "pdf"` metadata
   - Call `start_pdf_phase()` before each phase begins
   - Call `complete_pdf_phase()` after each phase completes

2. Phase mapping:
   - Before chunking → `start_pdf_phase(Chunking)`
   - After chunking → `complete_pdf_phase(Chunking)`, `start_pdf_phase(Extraction)`
   - After extraction → `complete_pdf_phase(Extraction)`, `start_pdf_phase(Embedding)`
   - After embedding → `complete_pdf_phase(Embedding)`, `start_pdf_phase(GraphStorage)`
   - After storage → `complete_pdf_phase(GraphStorage)`

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Break existing MD flow | Low | High | Conditional logic only for PDFs |
| Performance impact | Very Low | Low | Async calls, minimal overhead |
| Frontend display issues | Low | Medium | Test with Playwright E2E |
