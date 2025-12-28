# SOTA Ingestion Pipeline: Comprehensive Design Plan

> **Project:** EdgeQuake Knowledge Graph RAG System
> **Specification:** specs/19-ingestion-pipeline.md
> **Version:** 2.0.0
> **Date:** 2024-12-28
> **Updated:** 2024-12-28 (SOTA Prompt System Integration)
> **Status:** ✅ COMPLETE + ENHANCED

---

## Executive Summary

This document represents the complete design plan for upgrading EdgeQuake's ingestion pipeline to State-of-the-Art (SOTA) standards. The plan addresses all requirements outlined in the specification, providing comprehensive architecture documentation, data models, API contracts, implementation roadmap, and testing strategy.

### v2.0 Enhancements

- **SOTA Prompt System**: Full integration of LightRAG-style tuple-based extraction prompts
- **Roadblock Analysis**: Comprehensive risk mitigation strategies
- **Hybrid Migration Path**: Zero-disruption transition from JSON to tuple format
- **Citation System**: RAG responses with reference tracking

### Key Achievements

| Deliverable                   | Status       | Document                                               |
| ----------------------------- | ------------ | ------------------------------------------------------ |
| Current Architecture Analysis | ✅           | [01-architecture.md](01-architecture.md)               |
| Rust vs Python Comparison     | ✅           | [02-comparison.md](02-comparison.md)                   |
| SOTA Data Models              | ✅           | [03-data-models.md](03-data-models.md)                 |
| API Contracts                 | ✅           | [04-api-contracts.md](04-api-contracts.md)             |
| Implementation Plan           | ✅ **v2.0**  | [05-implementation-plan.md](05-implementation-plan.md) |
| Testing Strategy              | ✅           | [06-testing-strategy.md](06-testing-strategy.md)       |
| Prompt Comparison             | ✅           | [07-prompt-comparison.md](07-prompt-comparison.md)     |
| Documentation Crosscheck      | ✅           | [08-documentation-crosscheck.md](08-documentation-crosscheck.md) |

---

## Table of Contents

1. [Specification Coverage Matrix](#1-specification-coverage-matrix)
2. [SOTA Prompt System Highlights](#2-sota-prompt-system-highlights)
3. [Architecture Overview](#3-architecture-overview)
4. [Critical Gap Analysis](#4-critical-gap-analysis)
5. [Implementation Roadmap](#5-implementation-roadmap)
6. [Quick Reference](#6-quick-reference)
7. [Next Steps](#7-next-steps)

---

## 1. Specification Coverage Matrix

### 1.1 Functional Requirements (F-Series)

| ID   | Requirement                           | Priority | Status      | Document Reference                                       |
| ---- | ------------------------------------- | -------- | ----------- | -------------------------------------------------------- |
| F001 | Document ingestion endpoint           | P0       | ✅ Designed | [04-api-contracts.md#21](04-api-contracts.md)            |
| F002 | Chunk-level lineage with line numbers | P0       | ✅ Designed | [03-data-models.md#21](03-data-models.md)                |
| F003 | MapReduce description summarization   | P0       | ✅ Designed | [05-implementation-plan.md#5](05-implementation-plan.md) |
| F004 | Parallel chunk processing             | P0       | ✅ Designed | [05-implementation-plan.md#4](05-implementation-plan.md) |
| F005 | LLM response caching                  | P0       | ✅ Designed | [05-implementation-plan.md#5](05-implementation-plan.md) |
| F006 | Real-time progress tracking           | P0       | ✅ Designed | [03-data-models.md#3](03-data-models.md)                 |
| F007 | Cost tracking per operation           | P0       | ✅ Designed | [03-data-models.md#4](03-data-models.md)                 |
| F008 | Document suppression                  | P1       | ✅ Designed | [04-api-contracts.md#6](04-api-contracts.md)             |
| F009 | Entity CRUD operations                | P1       | ✅ Designed | [04-api-contracts.md#6](04-api-contracts.md)             |
| F010 | Gleaning multi-pass extraction        | P0       | ✅ Existing | [02-comparison.md](02-comparison.md)                     |
| F011 | Multi-tenant isolation                | P0       | ✅ Existing | [01-architecture.md](01-architecture.md)                 |
| F012 | WebSocket progress events             | P2       | ✅ Designed | [04-api-contracts.md#7](04-api-contracts.md)             |
| **F013** | **SOTA Prompt System**            | **P0**   | **✅ NEW**  | [05-implementation-plan.md#2](05-implementation-plan.md) |
| **F014** | **Citation/Reference Tracking**   | **P1**   | **✅ NEW**  | [05-implementation-plan.md#2](05-implementation-plan.md) |

### 1.2 Non-Functional Requirements (R-Series)

| ID   | Requirement                           | Status      | Implementation                            |
| ---- | ------------------------------------- | ----------- | ----------------------------------------- |
| R001 | Line number tracking in chunks        | ✅ Designed | Add `start_line`, `end_line` to TextChunk |
| R002 | Performance: <5s for 10KB document    | ✅ Designed | Parallel processing                       |
| R003 | Cost: <$0.01 per document average     | ✅ Designed | gpt-4o-mini, caching                      |
| R004 | Observability: full trace correlation | ✅ Designed | Lineage chain                             |
| R005 | Idempotent re-ingestion               | ✅ Designed | Content hash, upsert                      |
| R006 | Graceful degradation                  | ✅ Designed | Retry with fallback                       |
| **R007** | **Multi-language extraction**     | ✅ Designed | `{language}` parameter in prompts         |

### 1.3 Deliverables

| Deliverable                      | Status | Location                                               |
| -------------------------------- | ------ | ------------------------------------------------------ |
| Architecture diagrams            | ✅     | [01-architecture.md](01-architecture.md)               |
| Data model specifications        | ✅     | [03-data-models.md](03-data-models.md)                 |
| API contract definitions         | ✅     | [04-api-contracts.md](04-api-contracts.md)             |
| Implementation roadmap           | ✅     | [05-implementation-plan.md](05-implementation-plan.md) |
| Testing strategy                 | ✅     | [06-testing-strategy.md](06-testing-strategy.md)       |
| Comparison analysis              | ✅     | [02-comparison.md](02-comparison.md)                   |
| **SOTA Prompt Templates**        | ✅     | [05-implementation-plan.md#2](05-implementation-plan.md) |
| **Roadblock Analysis**           | ✅     | [05-implementation-plan.md#11](05-implementation-plan.md) |

---

## 2. SOTA Prompt System Highlights

### 2.1 Key Improvements

The updated plan integrates LightRAG's SOTA prompt system with the following enhancements:

| Feature                   | Before (JSON)                   | After (Tuple SOTA)                     |
| ------------------------- | ------------------------------- | -------------------------------------- |
| **Extraction Format**     | JSON with strict parsing        | Tuple `<\|#\|>` for robust parsing     |
| **Completion Detection**  | None                            | `<\|COMPLETE\|>` signal                |
| **Entity Naming**         | No guidance                     | Title case, consistent naming rules    |
| **N-ary Relationships**   | Not addressed                   | Explicit decomposition instructions    |
| **Multi-Language**        | English only                    | Configurable `{language}` parameter    |
| **Third Person**          | Not enforced                    | Required in system prompt              |
| **Examples**              | None                            | 3 comprehensive few-shot examples      |
| **Error Recovery**        | Parse failure = error           | Hybrid parser with fallback            |

### 2.2 Migration Strategy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     PROMPT MIGRATION PATH                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Phase 1: Dual Mode                                                         │
│  ══════════════════                                                         │
│  • HybridExtractionParser supports both JSON and Tuple                     │
│  • Feature flag: sota-prompts (default: enabled)                           │
│  • Feature flag: legacy-prompts (for rollback)                             │
│                                                                             │
│  Phase 2: Tuple Primary                                                     │
│  ═══════════════════════                                                    │
│  • Tuple format used for all new extractions                               │
│  • JSON parser as fallback for non-compliant LLM responses                 │
│  • Monitoring: Track format compliance rate                                │
│                                                                             │
│  Phase 3: Tuple Only (Future)                                              │
│  ════════════════════════════                                               │
│  • Remove JSON fallback after 95%+ compliance                              │
│  • Deprecate legacy-prompts feature flag                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.3 Roadblock Mitigation Summary

All identified roadblocks have mitigation strategies:

| Roadblock ID | Description                     | Mitigation                          | Status     |
| ------------ | ------------------------------- | ----------------------------------- | ---------- |
| RB-001       | LLM non-compliance with format  | Retry + JSON fallback               | ✅ Planned |
| RB-002       | System prompt variability       | Concatenation fallback              | ✅ Planned |
| RB-003       | Token limits for descriptions   | MapReduce summarization             | ✅ Planned |
| RB-004       | Entity name normalization       | Comprehensive normalization         | ✅ Planned |
| RB-005       | Parallel processing races       | Stateless extraction + semaphore    | ✅ Planned |
| RB-006       | WebSocket connection limits     | Connection pooling + limits         | ✅ Planned |

---

## 3. Architecture Overview

### 3.1 High-Level System Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SOTA INGESTION PIPELINE                              │
└─────────────────────────────────────────────────────────────────────────────┘

                              ┌─────────────────┐
                              │   Document      │
                              │   Upload        │
                              └────────┬────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 1: PREPROCESSING                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Content Hash │──│ Deduplication│──│ Text Extract │──│ Normalization│    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└────────────────────────────────────────────┬────────────────────────────────┘
                                             │
                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 2: CHUNKING (with line numbers)                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  Document Text ──▶ Chunks with (start_line, end_line, offsets)       │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Output: TextChunk { id, content, start_line, end_line, token_count }       │
└────────────────────────────────────────────┬────────────────────────────────┘
                                             │
                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 3: EXTRACTION (parallel with caching)                                │
│                                                                             │
│     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐           │
│     │ Chunk 1 │     │ Chunk 2 │     │ Chunk 3 │     │ Chunk N │           │
│     └────┬────┘     └────┬────┘     └────┬────┘     └────┬────┘           │
│          │               │               │               │                 │
│          ▼               ▼               ▼               ▼                 │
│     ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐           │
│     │ Cache   │     │ Cache   │     │ Cache   │     │ Cache   │           │
│     │ Check   │     │ Check   │     │ Check   │     │ Check   │           │
│     └────┬────┘     └────┬────┘     └────┬────┘     └────┬────┘           │
│          │               │               │               │                 │
│    hit   │   miss   hit  │   miss   hit  │   miss   hit  │   miss         │
│    ┌─────┴─────┐   ┌─────┴─────┐   ┌─────┴─────┐   ┌─────┴─────┐         │
│    │ Use Cache │   │ LLM Call  │   │ Use Cache │   │ LLM Call  │         │
│    └───────────┘   └───────────┘   └───────────┘   └───────────┘         │
│                                                                             │
│  Semaphore: max_concurrent_extractions = 4                                  │
└────────────────────────────────────────────┬────────────────────────────────┘
                                             │
                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 4: MERGING (with MapReduce summarization)                            │
│                                                                             │
│     Entity Mentions ──▶ Group by Normalized Name ──▶ Merge Descriptions    │
│                                                                             │
│     ┌───────────────────────────────────────────────────────────────────┐  │
│     │ IF descriptions.len() > threshold:                                │  │
│     │     Apply MapReduce Summarization                                 │  │
│     │ ELSE:                                                             │  │
│     │     Concatenate with separator                                    │  │
│     └───────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────┬────────────────────────────────┘
                                             │
                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 5: EMBEDDING                                                         │
│                                                                             │
│     Entities + Relationships ──▶ Generate Embeddings ──▶ Vector Storage    │
│                                                                             │
│     Model: text-embedding-3-small (1536 dimensions)                         │
└────────────────────────────────────────────┬────────────────────────────────┘
                                             │
                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 6: STORAGE (with full lineage)                                       │
│                                                                             │
│     ┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐ │
│     │   Graph Storage  │     │  Vector Storage  │     │ Lineage Storage  │ │
│     │   (PostgreSQL    │     │  (pgvector)      │     │ (PostgreSQL)     │ │
│     │    + AGE)        │     │                  │     │                  │ │
│     └──────────────────┘     └──────────────────┘     └──────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Component Architecture

See [01-architecture.md](01-architecture.md) for detailed component diagrams.

### 3.3 Data Flow

See [03-data-models.md](03-data-models.md) for complete data model specifications.

---

## 4. Critical Gap Analysis

### 4.1 Current State vs SOTA Requirements

| Feature                 | Current State | SOTA Requirement               | Gap Severity |
| ----------------------- | ------------- | ------------------------------ | ------------ |
| Line number tracking    | ❌ Missing    | Start/end line per chunk       | 🔴 High      |
| Parallel processing     | ❌ Sequential | Concurrent with semaphore      | 🔴 High      |
| MapReduce summarization | ❌ Missing    | LLM-based merge for large sets | 🔴 High      |
| LLM caching             | ❌ Missing    | Per-chunk response cache       | 🔴 High      |
| Progress tracking       | ⚠️ Partial    | Stage-level with events        | 🟡 Medium    |
| Cost tracking           | ⚠️ Basic      | Per-operation breakdown        | 🟡 Medium    |
| Lineage storage         | ❌ Missing    | Full chain tracking            | 🔴 High      |
| Document suppression    | ❌ Missing    | Cascade delete support         | 🟡 Medium    |

### 4.2 LightRAG Features to Port

From [02-comparison.md](02-comparison.md):

1. **Tuple-based extraction format** - More robust than JSON (✅ Added in v2.0)
2. **MapReduce summarization** - `_handle_entity_relation_summary()`
3. **LLM caching** - `llm_cache_list` per chunk
4. **History messages** - Progress tracking with message history
5. **Parallel processing** - `asyncio.Semaphore` pattern

---

## 5. Implementation Roadmap

### 5.1 Phase Overview

```
┌───────────────────────────────────────────────────────────────────────────┐
│                        IMPLEMENTATION TIMELINE                            │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  Week 1-2: PHASE 1 - Core Enhancements                                   │
│  ══════════════════════════════════                                       │
│  • Add line number tracking to TextChunk                                  │
│  • Implement parallel chunk processing                                    │
│  • Enhance token usage tracking                                           │
│                                                                           │
│  Week 2-3: PHASE 2 - MapReduce & Caching                                 │
│  ════════════════════════════════════════                                 │
│  • Create MapReduce summarizer                                            │
│  • Implement LLM response caching                                         │
│  • Integrate caching into extractor                                       │
│                                                                           │
│  Week 3-4: PHASE 3 - Progress & Cost Tracking                            │
│  ═════════════════════════════════════════════                            │
│  • Create progress tracking types                                         │
│  • Implement cost calculator                                              │
│  • Add event streaming                                                    │
│                                                                           │
│  Week 4-5: PHASE 4 - Lineage & Document Management                       │
│  ══════════════════════════════════════════════════                       │
│  • Create lineage storage                                                 │
│  • Implement document suppression                                         │
│  • Add cascade delete                                                     │
│                                                                           │
│  Week 5-6: PHASE 5 - API & Integration                                   │
│  ═════════════════════════════════════                                    │
│  • Add new API endpoints                                                  │
│  • Implement WebSocket handler                                            │
│  • Create E2E tests                                                       │
│                                                                           │
└───────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Detailed Tasks by Phase

See [05-implementation-plan.md](05-implementation-plan.md) for:

- Task breakdowns with effort estimates
- Code change specifications
- Migration strategy
- Risk assessment
- **NEW: SOTA Prompt System integration (Section 2)**
- **NEW: Roadblock Analysis (Section 11)**

### 5.3 File Modification Summary

| File            | Phase | Changes                                               |
| --------------- | ----- | ----------------------------------------------------- |
| `chunker.rs`    | 1     | +start_line, +end_line, +calculate_line_numbers()     |
| `pipeline.rs`   | 1, 2  | +extract_parallel(), +caching integration             |
| `extractor.rs`  | 1, 2  | +token tracking, +cache lookup, +SOTA prompts         |
| `merger.rs`     | 2     | +MapReduce integration                                |
| `summarizer.rs` | 2     | NEW: MapReduce summarizer                             |
| `cache.rs`      | 2     | NEW: LLM caching                                      |
| `progress.rs`   | 3     | NEW: Progress tracking                                |
| `cost.rs`       | 3     | NEW: Cost calculation                                 |
| `lineage.rs`    | 4     | NEW: Lineage storage                                  |
| `ws.rs`         | 5     | NEW: WebSocket handler                                |
| `prompts/`      | 1     | **NEW: SOTA prompt templates module**                 |

---

## 6. Quick Reference

### 6.1 Key Data Models

```rust
// Enhanced TextChunk
pub struct TextChunk {
    pub id: String,
    pub content: String,
    pub index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_line: usize,      // NEW: 1-based line number
    pub end_line: usize,        // NEW: inclusive
    pub token_count: usize,
    pub embedding: Option<Vec<f32>>,
}

// Lineage Chain
pub struct EntityLineageChain {
    pub entity_id: String,
    pub chunk_id: String,
    pub document_id: String,
    pub start_line: usize,
    pub end_line: usize,
    pub filename: String,
}

// Cost Breakdown
pub struct CostBreakdown {
    pub extraction: OperationCost,
    pub gleaning: OperationCost,
    pub summarization: OperationCost,
    pub embedding: OperationCost,
    pub total_usd: f64,
}
```

### 6.2 Key API Endpoints

| Method | Endpoint                         | Purpose           |
| ------ | -------------------------------- | ----------------- |
| POST   | `/api/v1/documents`              | Upload document   |
| GET    | `/api/v1/documents/track/{id}`   | Track progress    |
| GET    | `/api/v1/documents/{id}/lineage` | Get lineage       |
| DELETE | `/api/v1/documents/{id}`         | Suppress document |
| GET    | `/api/v1/costs/summary`          | Get cost summary  |
| WS     | `/api/v1/ws/progress/{id}`       | Real-time events  |

### 6.3 Configuration

```toml
[pipeline]
max_concurrent_extractions = 4
enable_caching = true
enable_mapreduce = true
force_llm_summary_on_merge = 6
context_size = 4000

[models]
extraction = "gpt-4o-mini"
embedding = "text-embedding-3-small"

[cost]
track_costs = true
alert_threshold_usd = 10.0
```

---

## 7. Next Steps

### 7.1 Immediate Actions

1. **Review and approve** this design plan
2. **Create feature branch** `feat/sota-ingestion-pipeline`
3. **Begin Phase 1** implementation
4. **Set up CI** for new test coverage requirements

### 7.2 Implementation Checklist

```
Phase 1: Core Enhancements + SOTA Prompts
- [ ] Add line numbers to TextChunk
- [ ] Implement calculate_line_numbers()
- [ ] Add parallel chunk processing
- [ ] Enhance token tracking
- [ ] Create prompts/mod.rs with EntityExtractionPrompts
- [ ] Implement TupleParser with <|#|> delimiter
- [ ] Add HybridExtractionParser for fallback
- [ ] Port LightRAG prompt templates
- [ ] Add entity name normalization
- [ ] Update unit tests

Phase 2: MapReduce & Caching
- [ ] Create summarizer.rs
- [ ] Create cache.rs
- [ ] Implement MemoryLLMCache
- [ ] Implement PostgreSQL cache
- [ ] Integrate caching into extractor
- [ ] Add rebuild from cache

Phase 3: Progress & Cost
- [ ] Create progress.rs
- [ ] Create cost.rs
- [ ] Implement ProgressTracker
- [ ] Implement CostCalculator
- [ ] Add event streaming

Phase 4: Lineage & Docs
- [ ] Create lineage types
- [ ] Implement lineage storage
- [ ] Add cascade delete
- [ ] Implement document suppression

Phase 5: API & Integration
- [ ] Add progress endpoints
- [ ] Add lineage endpoints
- [ ] Implement WebSocket
- [ ] Create E2E tests
- [ ] Update documentation
```

### 7.3 Success Metrics

| Metric               | Target | Validation        |
| -------------------- | ------ | ----------------- |
| Line number accuracy | 100%   | Unit tests        |
| Parallel speedup     | 3-4x   | Benchmarks        |
| Cache hit rate       | >50%   | Integration tests |
| Cost reduction       | 30%    | Real LLM tests    |
| Test coverage        | >80%   | CI pipeline       |

---

## Appendix A: Document Index

| Document                                               | Purpose                                 | Lines |
| ------------------------------------------------------ | --------------------------------------- | ----- |
| [01-architecture.md](01-architecture.md)               | System architecture with ASCII diagrams | ~400  |
| [02-comparison.md](02-comparison.md)                   | Rust vs Python feature comparison       | ~300  |
| [03-data-models.md](03-data-models.md)                 | Complete data model specifications      | ~500  |
| [04-api-contracts.md](04-api-contracts.md)             | API endpoint definitions                | ~400  |
| [05-implementation-plan.md](05-implementation-plan.md) | Phased implementation roadmap           | ~600  |
| [06-testing-strategy.md](06-testing-strategy.md)       | Test plans and strategies               | ~500  |
| [plan.md](plan.md)                                     | This document - master plan             | ~400  |

---

## Appendix B: Glossary

| Term             | Definition                                   |
| ---------------- | -------------------------------------------- |
| **Chunk**        | A segment of document text with metadata     |
| **Entity**       | A named concept extracted from text          |
| **Relationship** | A connection between two entities            |
| **Lineage**      | Tracking chain from entity to source         |
| **MapReduce**    | Divide-and-conquer summarization pattern     |
| **Gleaning**     | Multi-pass extraction for completeness       |
| **Suppression**  | Logical deletion of document and derivatives |
| **SOTA**         | State-of-the-Art                             |

---

## Appendix C: References

1. **LightRAG Paper**: https://arxiv.org/abs/2410.05779
2. **EdgeQuake Repository**: This repository
3. **Specification**: [specs/19-ingestion-pipeline.md](../specs/19-ingestion-pipeline.md)
4. **OpenAI Pricing**: https://openai.com/pricing
5. **PostgreSQL AGE**: https://age.apache.org/

---

_Document generated as part of EdgeQuake SOTA Ingestion Pipeline design initiative._
