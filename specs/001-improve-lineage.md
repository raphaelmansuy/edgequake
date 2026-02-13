# Mission: Comprehensive Lineage Extraction & Metadata Audit

## Task

You must start by FULLY Read this mission file before doing anything else. This is a critical safety mandate to prevent alignment drift. You can forget previous iterations, but never forget your mission.

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.

Your mission is to **conduct a deep audit of lineage extraction and metadata storage across the entire document ingestion pipeline** (Markdown + PDF), ensuring complete traceability from source documents to extracted entities. You must:

1. **Audit** existing metadata tracking at document and chunk levels
2. **Verify** that chunk_id, document_id, original PDF document id, filename, file_size, document_type, embedding_model, llm_model are correctly extracted and stored
3. **Ensure** each chunk contains the parent document id and full lineage chain
4. **Optimize** API endpoints for efficient metadata retrieval
5. **Enhance** WebUI to display complete lineage information with full traceability
6. **Update** all SDKs (Rust, TypeScript, Python, etc.) to retrieve and expose metadata
7. **Document** the complete lineage tracking architecture and usage

## Context

- **Location**: Lineage tracking spans multiple crates:
  - `edgequake/crates/edgequake-pipeline/src/lineage.rs` - Core lineage structures
  - `edgequake/crates/edgequake-core/src/types/{document,chunk}.rs` - Document/chunk types
  - `edgequake/crates/edgequake-storage/src/pdf_storage.rs` - PDF metadata storage
  - `edgequake/crates/edgequake-api/src/handlers/{documents,lineage}.rs` - API endpoints
  - `edgequake_webui/src/components/document/metadata-sidebar.tsx` - UI display
  - `edgequake_webui/src/types/lineage.ts` - TypeScript types
  - `sdks/{rust,typescript,python}/` - SDK implementations

- **Current State Analysis**:
  - ✅ Documents track basic metadata (id, file_path, status, created_at, content_length)
  - ✅ Chunks reference parent document via `full_doc_id`
  - ✅ PDF documents store comprehensive metadata (sha256, file_size, page_count, vision_model)
  - ✅ Lineage system tracks LLM/embedding providers (SPEC-032)
  - ⚠️ **Gap**: Chunk-level metadata may not include all required fields (embedding_model, llm_model used for that specific chunk)
  - ⚠️ **Gap**: Parent-child relationship from PDF → Document → Chunks may not be explicit in all queries
  - ⚠️ **Gap**: API may require multiple calls to retrieve complete lineage
  - ⚠️ **Gap**: WebUI may not display all available metadata fields
  - ⚠️ **Gap**: SDKs may not expose all lineage information

New requirement in detail document page:

- Have a deep audit of the Right Panel --> ensure the content is scrollable and all metadata is visible (file size, file type, document type, etc.). Currently it is not scrollable and some metadata is hidden.

Do the same of each Right Panel of the other pages (Dashboard, Entity, etc.) and ensure all are scrollable and the panel content is fixed to the right border of the container

Addition to the mission:

For Graph page -> Ensure the right panel is attached to the right border of the container and is scrollable. Currently, it is not attached to the right border and some content is hidden when the panel is resized to be smaller.

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**⚠️ CRITICAL: Re-read this mission file at the start of EVERY OODA iteration to avoid alignment drift.**

Mission file: `specs/001-improve-lineage.md`

You MUST always produce the 4 files per iteration, as shown below:

1. **observe.md** → Map the territory. Never make assumptions about code structure or function. Always verify against the actual codebase. When you don't know, go check the code or search on the web for answers and documentation.
2. **orient.md** → Analyze your findings and define possible solutions using First Principles as your north star. Assess risks and benefits of each approach.
3. **decide.md** → Prioritize specific changes to be made based on signal value and impact.
4. **act.md** → Implement the decided changes with precision, update the documentation, and reference specific file:line numbers and commit SHAs.

```
specs/001-improve-lineage/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   ├── observe.md
│   ├── orient.md
│   ├── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: `specs/001-improve-lineage.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Split large files** for maintainability, Use Single Responsibility Principle (SRP)
6. **Optimize** Rust build speed (latest toolchain)
7. **Document amendments** in WHY comments, high signal value, and precise terms in the codebase. Use ASCII diagrams where applicable.
8. **You must perform tests** and deliver evidence that all tests are passing after your changes.

**YOU MUST READ YOUR MISSION EVERY ITERATION!** It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know, make a search on the Web.

Always use **First Principle Thinking** as your north star.

---

## Deliverables

### 1. Comprehensive Audit Report

Document in `001-improve-lineage/ooda_loop/summary.md`:

- **Current State**: What metadata is tracked at each level (Document, PDF, Chunk, Entity)
- **Data Flow Diagram**: ASCII diagram showing metadata propagation through pipeline
- **Gaps Identified**: Missing fields, incomplete tracking, performance issues
- **Recommendations**: Prioritized improvements with effort/impact analysis

### 2. Enhanced Metadata Tracking

Ensure these fields are tracked and retrievable:

| Level        | Required Metadata                                                                |
| ------------ | -------------------------------------------------------------------------------- |
| **Document** | document_id, file_path, file_size, document_type, created_at, processed_at       |
| **PDF**      | pdf_id, document_id, filename, file_size_bytes, sha256_checksum, page_count      |
| **Chunk**    | chunk_id, parent_document_id, chunk_index, start_line, end_line, tokens          |
| **Lineage**  | extraction_provider, extraction_model, embedding_provider, embedding_model, dims |
| **Entity**   | entity_id, chunk_ids, source_documents, extraction_metadata                      |

### 3. Optimized API Endpoints

Create/optimize these endpoints for efficient metadata retrieval:

```
GET /api/v1/documents/:id/lineage         # Complete lineage for document
GET /api/v1/documents/:id/metadata        # All metadata in single response
GET /api/v1/chunks/:id/lineage            # Chunk lineage with parent refs
GET /api/v1/entities/:id/provenance       # Entity source traceability
```

**Performance target**: Single API call retrieves complete lineage tree (no N+1 queries).

### 4. Enhanced WebUI Display

Update `MetadataSidebar` and related components to show:

- **Document Lineage Tree**: Document → PDF → Chunks → Entities (visual hierarchy)
- **Processing Pipeline**: Each stage with model used, timestamp, duration
- **Source Traceability**: Click entity → see source chunks → see source PDF/document
- **Metadata Grid**: All fields in organized, searchable format
- **Export Capability**: Download complete lineage as JSON/CSV

### 5. Updated SDK Implementations

Update all SDKs to expose lineage methods:

```rust
// Rust SDK
let lineage = client.documents().get_lineage(&doc_id).await?;
let metadata = client.documents().get_metadata(&doc_id).await?;
let chunk_lineage = client.chunks().get_lineage(&chunk_id).await?;
```

```typescript
// TypeScript SDK
const lineage = await client.documents.getLineage(docId);
const metadata = await client.documents.getMetadata(docId);
const chunkLineage = await client.chunks.getLineage(chunkId);
```

### 6. Comprehensive Documentation

Update these docs:

- `docs/architecture/lineage-tracking.md` - Complete lineage architecture
- `docs/api-reference/lineage-endpoints.md` - API documentation with examples
- `docs/tutorials/tracing-entity-sources.md` - Tutorial for users
- `docs/operations/metadata-debugging.md` - How to debug lineage issues
- API docs in `edgequake-api/src/handlers/` - OpenAPI/utoipa annotations

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → incomplete implementation → user frustration → unreliable system.

---

## Success Criteria

### Functional Requirements

- [ ] **F1**: All document metadata (id, file_path, file_size, type) is stored at document level
- [ ] **F2**: All PDF metadata (pdf_id, document_id, filename, checksum) is stored and linked
- [ ] **F3**: Every chunk contains parent_document_id and complete position info
- [ ] **F4**: LLM and embedding models are tracked at document and chunk level
- [ ] **F5**: Single API call retrieves complete document lineage tree
- [ ] **F6**: WebUI displays all lineage information in organized hierarchy
- [ ] **F7**: All SDKs expose lineage retrieval methods
- [ ] **F8**: PDF → Document → Chunk → Entity chain is traceable in both directions

### Technical Requirements

- [ ] **T1**: API response time for lineage query < 200ms (95th percentile)
- [ ] **T2**: No N+1 query problems in lineage retrieval
- [ ] **T3**: Lineage data is indexed for fast lookup
- [ ] **T4**: All metadata is validated before storage
- [ ] **T5**: Backward compatibility maintained for existing documents
- [ ] **T6**: All tests pass (unit, integration, E2E)
- [ ] **T7**: No clippy warnings in modified code
- [ ] **T8**: Documentation is complete and accurate

### Quality Requirements

- [ ] **Q1**: Code follows Single Responsibility Principle
- [ ] **Q2**: ASCII diagrams illustrate complex flows
- [ ] **Q3**: WHY comments explain design decisions
- [ ] **Q4**: Error messages are actionable
- [ ] **Q5**: API follows REST best practices
- [ ] **Q6**: WebUI is responsive and accessible
- [x] **Q6a**: Detail page right panel scrollable with all metadata visible
- [x] **Q6b**: Graph page right panel attached to right border and scrollable
- [x] **Q6c**: Documents page: all buttons have aria-labels (52 → 0 violations)
- [x] **Q6d**: Documents page: table has proper ARIA semantics
- [x] **Q6e**: Documents page: responsive at 375px mobile and 768px tablet
- [x] **Q6f**: Graph page right panel: no horizontal content overflow (Radix table wrapper override)
- [x] **Q6g**: Graph page PropertyValue: proper flex shrinking with min-w-0 and reduced gap
- [x] **Q6h**: Scrollable area padding audit: dashboard recent-activity (0px→4px), entity browser (6px→8px)
- [x] **Q6i**: Graph page: description text uses break-words for long content
- [ ] **Q7**: Documentation includes real examples
- [ ] **Q8**: Breaking changes are documented in CHANGELOG

---

## Key Investigation Areas

### 1. Document-Level Metadata

**File**: `edgequake/crates/edgequake-core/src/types/document.rs`

**Questions**:

- Are all required fields being populated during ingestion?
- Is `metadata: Option<serde_json::Value>` being used effectively?
- Should we add type-safe fields vs. using JSON blob?
- Is `content_length` always set? Is it in bytes or characters?
- Do we track original file checksum (SHA-256) at document level?

### 2. PDF-to-Document Linkage

**Files**:

- `edgequake/crates/edgequake-storage/src/pdf_storage.rs`
- `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

**Questions**:

- Is `PdfDocument.document_id` always set after processing?
- Can we query "get all chunks from PDF X" efficiently?
- Is the PDF → Document → Chunks lineage bidirectional?
- Are PDF metadata fields exposed in document metadata?
- Can we trace a chunk back to its original PDF page?

### 3. Chunk-Level Metadata

**Files**:

- `edgequake/crates/edgequake-core/src/types/chunk.rs`
- `edgequake/crates/edgequake-pipeline/src/lineage.rs`

**Questions**:

- Does every chunk store the parent `full_doc_id`?
- Are `start_line`, `end_line`, `start_offset`, `end_offset` always populated?
- Is the chunk index (`chunk_order_index`) reliable for ordering?
- Do we track which embedding model was used for each chunk?
- Is chunk metadata stored in vector storage for retrieval?

### 4. Lineage System

**File**: `edgequake/crates/edgequake-pipeline/src/lineage.rs`

**Questions**:

- Is lineage tracking enabled by default or opt-in?
- Are provider details (LLM/embedding) stored for each extraction?
- Can we query lineage by entity to find all source chunks?
- Is lineage data persisted in KV storage or only in memory?
- Are there performance issues with lineage tracking on large documents?

### 5. API Efficiency

**Files**:

- `edgequake/crates/edgequake-api/src/handlers/documents.rs`
- `edgequake/crates/edgequake-api/src/handlers/lineage.rs`

**Questions**:

- How many API calls are needed to get complete lineage?
- Are there N+1 query problems in current implementation?
- Can we batch-fetch chunk metadata efficiently?
- Should we add a dedicated `/lineage` endpoint?
- Are responses properly cached?

### 6. WebUI Display

**Files**:

- `edgequake_webui/src/components/document/metadata-sidebar.tsx`
- `edgequake_webui/src/types/lineage.ts`

**Questions**:

- Does the UI show all available metadata fields?
- Is the lineage tree visualization intuitive?
- Can users click through the hierarchy (Doc → PDF → Chunks → Entities)?
- Are loading states handled for slow metadata fetches?
- Is metadata searchable/filterable?

### 7. SDK Completeness

**Files**:

- `sdks/rust/src/resources/documents.rs`
- `sdks/typescript/src/resources/documents.ts`
- `sdks/python/edgequake/resources/documents.py`

**Questions**:

- Do SDKs expose lineage retrieval methods?
- Are response types properly typed (TypeScript, Rust)?
- Do SDKs handle pagination for large lineage trees?
- Is there a unified lineage interface across languages?
- Are SDK docs up to date with new endpoints?

---

## Architectural Principles

### 1. Data Integrity

```
┌─────────────────────────────────────────────────────────────────┐
│                     LINEAGE INTEGRITY CHAIN                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  PDF Upload                                                     │
│    ↓                                                           │
│  PDF Storage (pdf_id, sha256, file_size) ─────────────────┐   │
│    ↓                                                       │   │
│  Document Creation (document_id, pdf_id, content) ────────┼─→ │
│    ↓                                                       │   │
│  Chunking (chunk_id, parent_doc_id, positions) ───────────┼─→ │
│    ↓                                                       │   │
│  Extraction (entity_id, chunk_ids, llm_model) ────────────┼─→ │
│    ↓                                                       │   │
│  Lineage Storage (complete graph with all refs) ──────────┘   │
│                                                                 │
│  INVARIANTS:                                                    │
│  - Every chunk MUST have valid parent document_id               │
│  - Every entity MUST reference at least one chunk               │
│  - Every document with PDF MUST link back to pdf_id             │
│  - All timestamps MUST be UTC ISO-8601                          │
│  - All IDs MUST be immutable once created                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2. Query Efficiency

- **Principle**: Complete lineage retrieval in O(1) database queries
- **Implementation**: Denormalize lineage data for fast reads
- **Trade-off**: Slightly larger storage for significantly faster queries
- **Monitoring**: Track P95 query latency, alert if > 200ms

### 3. Backward Compatibility

- **Principle**: Never break existing document metadata
- **Implementation**: Add new fields as optional, migrate lazily
- **Migration**: Background job to backfill missing metadata
- **Rollback**: Always support reading old format

### 4. Error Handling

- **Principle**: Lineage gaps should be visible, not silent
- **Implementation**: Explicit `None`/`null` for missing data vs. omitted fields
- **Logging**: Warn when expected metadata is missing
- **Recovery**: Provide admin tools to repair broken lineage

---

## Testing Strategy

### Unit Tests

- [ ] Test document metadata serialization/deserialization
- [ ] Test chunk parent_id linkage
- [ ] Test lineage builder correctness
- [ ] Test API response schema validation
- [ ] Test SDK types match API responses

### Integration Tests

- [ ] Test PDF → Document → Chunks → Entities full pipeline
- [ ] Test lineage API returns complete tree
- [ ] Test metadata updates propagate correctly
- [ ] Test concurrent document processing doesn't break lineage
- [ ] Test large document lineage performance

### E2E Tests

- [ ] Upload PDF → verify metadata in UI
- [ ] Click entity → trace back to source PDF page
- [ ] Query lineage API → verify all fields present
- [ ] Test SDK lineage retrieval from all languages
- [ ] Test export lineage as JSON

### Performance Tests

- [ ] Benchmark lineage query latency (target: P95 < 200ms)
- [ ] Test with 1000+ chunk documents
- [ ] Measure API response size (should be < 1MB for typical doc)
- [ ] Profile N+1 queries in lineage retrieval

---

## Migration Plan

### Phase 1: Audit (Iterations 1-10)

- Map existing metadata tracking
- Identify all gaps
- Document current API behavior
- Create test suite for current state

### Phase 2: Foundation (Iterations 11-20)

- Ensure all required fields exist in types
- Add database indexes for lineage queries
- Create efficient API endpoints
- Update lineage storage logic

### Phase 3: API Enhancement (Iterations 21-30)

- Optimize lineage retrieval queries
- Add batch endpoints
- Implement caching
- Update OpenAPI docs

### Phase 4: UI/SDK Updates (Iterations 31-40)

- Enhance WebUI metadata display
- Update all SDKs
- Create tutorials and examples
- Test E2E flows

### Phase 5: Validation (Iterations 41-50)

- Run performance benchmarks
- Fix any discovered issues
- Complete documentation
- Prepare migration guide

---

## Risk Mitigation

### Risk 1: Breaking Changes

**Mitigation**:

- Maintain backward compatibility
- Version API endpoints if needed
- Provide migration scripts
- Document breaking changes in CHANGELOG

### Risk 2: Performance Degradation

**Mitigation**:

- Benchmark before/after changes
- Add database indexes proactively
- Implement query caching
- Monitor production metrics

### Risk 3: Incomplete Lineage Data

**Mitigation**:

- Validate all metadata before storage
- Add missing field detection
- Provide repair tools
- Log warnings (not errors) for gaps

### Risk 4: Database Migration Failures

**Mitigation**:

- Test migrations on copy of production data
- Make migrations idempotent
- Support rollback
- Monitor migration progress

---

## References

### Code References

- `edgequake/crates/edgequake-pipeline/src/lineage.rs` - Lineage tracking implementation
- `edgequake/crates/edgequake-core/src/types/` - Core type definitions
- `edgequake/crates/edgequake-storage/src/pdf_storage.rs` - PDF metadata storage
- `edgequake/crates/edgequake-api/src/handlers/lineage.rs` - Lineage API
- `edgequake_webui/src/components/document/metadata-sidebar.tsx` - UI display
- `docs/architecture/lineage-tracking.md` - Architecture docs (to be created)

### Specifications

- **SPEC-032**: Workspace-specific LLM/embedding providers
- **SPEC-002**: Unified Ingestion Pipeline
- **SPEC-007**: PDF Upload Support with Vision LLM
- **FEAT0011**: Document-Chunk-Entity Lineage tracking
- **FEAT0019**: Source span tracking with line numbers
- **BR0019**: Source spans must include line numbers
- **BR0701**: Lineage preserved for all entities

### External Resources

Search these topics if needed:

- "metadata tracking in document processing systems"
- "lineage tracking in data pipelines best practices"
- "efficient graph queries for provenance tracking"
- "REST API design for hierarchical data"
- "PostgreSQL indexing for lineage queries"

---

## OODA Loop Workflow

### Each Iteration MUST:

1. **START**: Re-read this entire mission file
2. **OBSERVE**: Examine actual code, run tests, gather facts
3. **ORIENT**: Analyze gaps, evaluate solutions, assess risks
4. **DECIDE**: Choose specific changes based on impact/effort
5. **ACT**: Implement, test, document, commit with "OODA-XX: summary"
6. **LOG**: Create all 4 files (observe, orient, decide, act) in `ooda_loop/iteration_XX/`
7. **VERIFY**: Run tests, check clippy, validate changes
8. **NEXT**: Update summary.md, move to next iteration

### Output Format for Each File

**observe.md**:

```markdown
# Observation - Iteration XX

## Files Examined

- file1.rs (lines 1-100) - found X
- file2.ts (lines 50-150) - discovered Y

## Tests Run

- cargo test --crate edgequake-pipeline
- Results: X passing, Y failing

## Current State

- [Fact 1 from codebase]
- [Fact 2 from codebase]
```

**orient.md**:

```markdown
# Analysis - Iteration XX

## Gaps Identified

1. [Gap description]
2. [Gap description]

## Possible Solutions

### Solution A

- Pros: ...
- Cons: ...
- Risk: Low/Medium/High

### Solution B

- Pros: ...
- Cons: ...
- Risk: Low/Medium/High

## Recommendation

[Chosen solution with justification]
```

**decide.md**:

```markdown
# Decision - Iteration XX

## Changes to Make

1. [Specific change with file:line reference]
2. [Specific change with file:line reference]

## Priority

1. High impact, low effort
2. High impact, high effort
   ...

## Expected Outcome

[What will be different after implementation]
```

**act.md**:

```markdown
# Implementation - Iteration XX

## Changes Made

1. File: edgequake-pipeline/src/lineage.rs
   - Lines: 100-150
   - Change: Added embedding_model field
   - Commit: abcd1234

## Tests Added/Updated

- test_lineage_with_embedding_model (passing)
- test_backward_compatibility (passing)

## Documentation Updated

- docs/architecture/lineage-tracking.md - Section 3.2

## Verification

- cargo test: ✅ All pass
- cargo clippy: ✅ No warnings
```

---

## Final Notes

This mission is **not complete** until:

1. ✅ All success criteria are met (F1-F8, T1-T8, Q1-Q8)
2. ✅ All tests pass (unit, integration, E2E, performance)
3. ✅ All documentation is updated and accurate
4. ✅ All SDKs expose complete lineage functionality
5. ✅ WebUI displays all metadata in organized manner
6. ✅ API is optimized (no N+1 queries, < 200ms P95 latency)
7. ✅ Migration plan is documented
8. ✅ Risks are mitigated

**Remember**: Re-read this mission file at the start of EVERY iteration. Alignment drift is the enemy of mission success.

---

## Contact and Escalation

If you encounter:

- **Architectural decisions** beyond lineage tracking → Document in orient.md, seek user input
- **Breaking API changes** required → Document alternatives, propose migration path
- **Performance blockers** that can't be resolved → Document bottleneck, propose workaround
- **Conflicting requirements** → Document conflict, propose resolution

**Never**: Make assumptions. Always verify against actual code. Always use First Principles thinking.

---

Ensure there is e2e test for metadata for each SDK.

I use playwrigth to test metadata display in WebUI. I will check that all metadata fields are displayed correctly and that lineage information is accurate.

**Mission Status**: 🔄 PHASE 5 (Validation — Iterations 41-60)

**Latest Progress** (OODA 41-50):

- ✅ Detail page right panel scrollability fixed (`metadata-sidebar.tsx`: `min-h-0` + `overflow-hidden`)
- ✅ Graph page right panel verified correct (already attached to right border, already scrollable)
- ✅ Documents page accessibility audit: 52 icon-only buttons without `aria-label` → fixed to 0
- ✅ Documents page table semantics: `aria-label`, `scope="col"`, sr-only Actions header
- ✅ Documents page responsive audit: 375px mobile + 768px tablet verified functional
- ✅ Search input `aria-label="Search documents"` added
- ✅ Pagination buttons labeled ("First page", "Previous page", "Next page", "Last page")

**Latest Progress** (OODA 51-60):

- ✅ Graph panel horizontal content overflow eliminated: Radix ScrollArea `display: table` wrapper overridden to `display: block` via Tailwind `[&_[data-slot=scroll-area-viewport]>div]:!block`
- ✅ PropertyValue component: removed `min-w-[70px]` label constraint, added `min-w-0` to outer div, reduced gap from `gap-3` to `gap-2`, value span uses `truncate min-w-0` for proper flex shrinking
- ✅ Description paragraph: added `break-words` for proper text wrapping in narrow panels
- ✅ Content wrapper: added `overflow-hidden` inside ScrollArea to clip any remaining overflow
- ✅ Scrollable area padding audit completed across all pages (dashboard, graph, query, pipeline, documents)
- ✅ Dashboard recent-activity: padding added (0px → 4px top/bottom via `py-1`)
- ✅ Entity browser: vertical padding increased (6px → 8px via `py-2 px-1.5`)
- ✅ DOM hierarchy verified: viewport scrollWidth === clientWidth (279px = 279px, zero horizontal overflow)

**Next Step**: Final validation, performance benchmarks, and documentation completion.
