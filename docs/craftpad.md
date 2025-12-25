# EdgeQuake Documentation Sync - Working Notes

## Last Updated: 2025-12-25T12:00:00Z
## Current Phase: update
## Current File: Phase 4 updates complete

### Completed
- [x] Phase 1: Inventory - All source files and docs mapped
- [x] Read all documentation files (0001-0008)
- [x] Read backend source code (orchestrator, query engine, pipeline, extractor, merger)
- [x] Read WebUI API client and types
- [x] Phase 2: Analysis complete
- [x] Phase 3: No files to archive (all current)
- [x] Phase 4: Documentation updates

### Updates Made

#### 0002-architecture-overview.md
- Fixed Next.js version: "Next.js 16" → "Next.js 15" (Next.js doesn't have v16)
- Fixed WebUI technology table: "Next.js 16" → "Next.js 15"

#### 0003-api-reference.md
- Added Ollama Emulation API section with full documentation:
  - Query mode prefixes (/local, /global, /naive, /hybrid, /mix, /bypass, /context)
  - GET /api/version
  - GET /api/tags
  - GET /api/ps
  - POST /api/generate
  - POST /api/chat
- Added POST /api/v1/documents/scan endpoint
- Added POST /api/v1/documents/reprocess endpoint
- Updated Table of Contents to include Ollama API
- Updated Base URL Structure to show Ollama API routes

#### docs/README.md
- Added link to new Algorithms Reference document

#### NEW: 0009-algorithms-reference.md
Created comprehensive algorithms documentation covering:
- Document Ingestion Pipeline (6 stages)
- Chunking Algorithm (token-based with overlap)
- Entity Extraction Algorithm (LLM-based structured extraction)
- Gleaning Algorithm (multi-pass extraction for better coverage)
- Entity Normalization Algorithm (UPPERCASE_UNDERSCORE format)
- Knowledge Graph Merging Algorithm (deduplication, description aggregation)
- Query Modes (6 modes: Naive, Local, Global, Hybrid, Mix, Bypass)
- Context Retrieval Algorithm
- Context Truncation Algorithm
- Token Budget Management
- Performance Characteristics
- Best Practices

### Verified Findings

#### QueryMode Consistency
- `edgequake-core/src/types/query.rs` has 6 modes (including Bypass) - CANONICAL
- `edgequake-query/src/modes.rs` has 5 modes (missing Bypass) - NEEDS CODE FIX
- Documentation is CORRECT - references core types which has all 6 modes

### Pending Actions
- [x] Fix Next.js version reference
- [x] Add Ollama emulation API documentation
- [x] Add document scan and reprocess endpoints
- [x] Document gleaning algorithm in detail
- [x] Document entity normalization algorithm
- [x] Document context truncation algorithm
- [x] Add conversation history support to query documentation
- [x] Update algorithm documentation with precision
- [ ] Note for code team: edgequake-query/src/modes.rs missing Bypass mode

