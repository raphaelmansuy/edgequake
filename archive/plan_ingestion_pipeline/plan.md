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

| Deliverable                   | Status      | Document                                                           |
| ----------------------------- | ----------- | ------------------------------------------------------------------ |
| Current Architecture Analysis | ✅          | [01-architecture.md](01-architecture.md)                           |
| Rust vs Python Comparison     | ✅          | [02-comparison.md](02-comparison.md)                               |
| SOTA Data Models              | ✅          | [03-data-models.md](03-data-models.md)                             |
| API Contracts                 | ✅          | [04-api-contracts.md](04-api-contracts.md)                         |
| Implementation Plan           | ✅ **v2.0** | [05-implementation-plan.md](05-implementation-plan.md)             |
| Testing Strategy              | ✅          | [06-testing-strategy.md](06-testing-strategy.md)                   |
| Prompt Comparison             | ✅          | [07-prompt-comparison.md](07-prompt-comparison.md)                 |
| Documentation Crosscheck      | ✅          | [08-documentation-crosscheck.md](08-documentation-crosscheck.md)   |
| **WebUI Architecture**        | ✅ **NEW**  | [10-webui-spec-architecture.md](10-webui-spec-architecture.md)     |
| **WebUI Screen Flows**        | ✅ **NEW**  | [11-webui-screen-flows.md](11-webui-screen-flows.md)               |
| **WebUI API Integration**     | ✅ **NEW**  | [12-webui-api-integration.md](12-webui-api-integration.md)         |
| **WebUI Components**          | ✅ **NEW**  | [13-webui-components.md](13-webui-components.md)                   |
| **WebUI WebSocket Progress**  | ✅ **NEW**  | [14-webui-websocket-progress.md](14-webui-websocket-progress.md)   |
| **WebUI Lineage Viz**         | ✅ **NEW**  | [15-webui-lineage-viz.md](15-webui-lineage-viz.md)                 |
| **WebUI Cost Monitoring**     | ✅ **NEW**  | [16-webui-cost-monitoring.md](16-webui-cost-monitoring.md)         |
| **WebUI Implementation Plan** | ✅ **NEW**  | [17-webui-implementation-plan.md](17-webui-implementation-plan.md) |

---

## Table of Contents

1. [Specification Coverage Matrix](#1-specification-coverage-matrix)
2. [SOTA Prompt System Highlights](#2-sota-prompt-system-highlights)
3. [Architecture Overview](#3-architecture-overview)
4. [Critical Gap Analysis](#4-critical-gap-analysis)
5. [Implementation Roadmap](#5-implementation-roadmap)
6. [Quick Reference](#6-quick-reference)
7. [WebUI Specification](#7-webui-specification)
8. [Layout Architecture Verification](#8-layout-architecture-verification-v21)
9. [Next Steps](#9-next-steps)

---

## 1. Specification Coverage Matrix

### 1.1 Functional Requirements (F-Series)

| ID       | Requirement                           | Priority | Status      | Document Reference                                       |
| -------- | ------------------------------------- | -------- | ----------- | -------------------------------------------------------- |
| F001     | Document ingestion endpoint           | P0       | ✅ Designed | [04-api-contracts.md#21](04-api-contracts.md)            |
| F002     | Chunk-level lineage with line numbers | P0       | ✅ Designed | [03-data-models.md#21](03-data-models.md)                |
| F003     | MapReduce description summarization   | P0       | ✅ Designed | [05-implementation-plan.md#5](05-implementation-plan.md) |
| F004     | Parallel chunk processing             | P0       | ✅ Designed | [05-implementation-plan.md#4](05-implementation-plan.md) |
| F005     | LLM response caching                  | P0       | ✅ Designed | [05-implementation-plan.md#5](05-implementation-plan.md) |
| F006     | Real-time progress tracking           | P0       | ✅ Designed | [03-data-models.md#3](03-data-models.md)                 |
| F007     | Cost tracking per operation           | P0       | ✅ Designed | [03-data-models.md#4](03-data-models.md)                 |
| F008     | Document suppression                  | P1       | ✅ Designed | [04-api-contracts.md#6](04-api-contracts.md)             |
| F009     | Entity CRUD operations                | P1       | ✅ Designed | [04-api-contracts.md#6](04-api-contracts.md)             |
| F010     | Gleaning multi-pass extraction        | P0       | ✅ Existing | [02-comparison.md](02-comparison.md)                     |
| F011     | Multi-tenant isolation                | P0       | ✅ Existing | [01-architecture.md](01-architecture.md)                 |
| F012     | WebSocket progress events             | P2       | ✅ Designed | [04-api-contracts.md#7](04-api-contracts.md)             |
| **F013** | **SOTA Prompt System**                | **P0**   | **✅ NEW**  | [05-implementation-plan.md#2](05-implementation-plan.md) |
| **F014** | **Citation/Reference Tracking**       | **P1**   | **✅ NEW**  | [05-implementation-plan.md#2](05-implementation-plan.md) |

### 1.2 Non-Functional Requirements (R-Series)

| ID       | Requirement                           | Status      | Implementation                            |
| -------- | ------------------------------------- | ----------- | ----------------------------------------- |
| R001     | Line number tracking in chunks        | ✅ Designed | Add `start_line`, `end_line` to TextChunk |
| R002     | Performance: <5s for 10KB document    | ✅ Designed | Parallel processing                       |
| R003     | Cost: <$0.01 per document average     | ✅ Designed | gpt-4o-mini, caching                      |
| R004     | Observability: full trace correlation | ✅ Designed | Lineage chain                             |
| R005     | Idempotent re-ingestion               | ✅ Designed | Content hash, upsert                      |
| R006     | Graceful degradation                  | ✅ Designed | Retry with fallback                       |
| **R007** | **Multi-language extraction**         | ✅ Designed | `{language}` parameter in prompts         |

### 1.3 Deliverables

| Deliverable               | Status | Location                                                  |
| ------------------------- | ------ | --------------------------------------------------------- |
| Architecture diagrams     | ✅     | [01-architecture.md](01-architecture.md)                  |
| Data model specifications | ✅     | [03-data-models.md](03-data-models.md)                    |
| API contract definitions  | ✅     | [04-api-contracts.md](04-api-contracts.md)                |
| Implementation roadmap    | ✅     | [05-implementation-plan.md](05-implementation-plan.md)    |
| Testing strategy          | ✅     | [06-testing-strategy.md](06-testing-strategy.md)          |
| Comparison analysis       | ✅     | [02-comparison.md](02-comparison.md)                      |
| **SOTA Prompt Templates** | ✅     | [05-implementation-plan.md#2](05-implementation-plan.md)  |
| **Roadblock Analysis**    | ✅     | [05-implementation-plan.md#11](05-implementation-plan.md) |

---

## 2. SOTA Prompt System Highlights

### 2.1 Key Improvements

The updated plan integrates LightRAG's SOTA prompt system with the following enhancements:

| Feature                  | Before (JSON)            | After (Tuple SOTA)                  |
| ------------------------ | ------------------------ | ----------------------------------- |
| **Extraction Format**    | JSON with strict parsing | Tuple `<\|#\|>` for robust parsing  |
| **Completion Detection** | None                     | `<\|COMPLETE\|>` signal             |
| **Entity Naming**        | No guidance              | Title case, consistent naming rules |
| **N-ary Relationships**  | Not addressed            | Explicit decomposition instructions |
| **Multi-Language**       | English only             | Configurable `{language}` parameter |
| **Third Person**         | Not enforced             | Required in system prompt           |
| **Examples**             | None                     | 3 comprehensive few-shot examples   |
| **Error Recovery**       | Parse failure = error    | Hybrid parser with fallback         |

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

| Roadblock ID | Description                    | Mitigation                       | Status     |
| ------------ | ------------------------------ | -------------------------------- | ---------- |
| RB-001       | LLM non-compliance with format | Retry + JSON fallback            | ✅ Planned |
| RB-002       | System prompt variability      | Concatenation fallback           | ✅ Planned |
| RB-003       | Token limits for descriptions  | MapReduce summarization          | ✅ Planned |
| RB-004       | Entity name normalization      | Comprehensive normalization      | ✅ Planned |
| RB-005       | Parallel processing races      | Stateless extraction + semaphore | ✅ Planned |
| RB-006       | WebSocket connection limits    | Connection pooling + limits      | ✅ Planned |

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

| File            | Phase | Changes                                           |
| --------------- | ----- | ------------------------------------------------- |
| `chunker.rs`    | 1     | +start_line, +end_line, +calculate_line_numbers() |
| `pipeline.rs`   | 1, 2  | +extract_parallel(), +caching integration         |
| `extractor.rs`  | 1, 2  | +token tracking, +cache lookup, +SOTA prompts     |
| `merger.rs`     | 2     | +MapReduce integration                            |
| `summarizer.rs` | 2     | NEW: MapReduce summarizer                         |
| `cache.rs`      | 2     | NEW: LLM caching                                  |
| `progress.rs`   | 3     | NEW: Progress tracking                            |
| `cost.rs`       | 3     | NEW: Cost calculation                             |
| `lineage.rs`    | 4     | NEW: Lineage storage                              |
| `ws.rs`         | 5     | NEW: WebSocket handler                            |
| `prompts/`      | 1     | **NEW: SOTA prompt templates module**             |

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

## 7. WebUI Specification

### 7.1 Overview

The WebUI specification covers the frontend implementation required to expose the new ingestion pipeline features to end users. This includes real-time progress tracking, lineage visualization, and cost monitoring.

### 7.2 Specification Documents

| Document                                                         | Purpose               | Key Features                                            |
| ---------------------------------------------------------------- | --------------------- | ------------------------------------------------------- |
| [10-webui-spec-architecture.md](10-webui-spec-architecture.md)   | Overall architecture  | Component hierarchy, state management, technology stack |
| [11-webui-screen-flows.md](11-webui-screen-flows.md)             | UI wireframes         | 7 main screens with ASCII wireframes                    |
| [12-webui-api-integration.md](12-webui-api-integration.md)       | API integration       | TypeScript types, React Query hooks, WebSocket client   |
| [13-webui-components.md](13-webui-components.md)                 | Component specs       | 12 new/updated components with props                    |
| [14-webui-websocket-progress.md](14-webui-websocket-progress.md) | Real-time progress    | WebSocket protocol, state management, reconnection      |
| [15-webui-lineage-viz.md](15-webui-lineage-viz.md)               | Lineage visualization | Tree/graph/table views, interactive features            |
| [16-webui-cost-monitoring.md](16-webui-cost-monitoring.md)       | Cost tracking UI      | Dashboard, budget management, reporting                 |

### 7.3 Key WebUI Features

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     WEBUI FEATURE MATRIX                                    │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────┬───────────────────────────────────────────────────┐
│ Feature                 │ Description                                       │
├─────────────────────────┼───────────────────────────────────────────────────┤
│ Real-Time Progress      │ WebSocket-powered stage-by-stage progress        │
│                         │ Live ETA, cancel/pause controls                  │
├─────────────────────────┼───────────────────────────────────────────────────┤
│ Lineage Visualization   │ Tree, graph, and table views                     │
│                         │ Document → Chunk → Entity provenance chain       │
├─────────────────────────┼───────────────────────────────────────────────────┤
│ Cost Monitoring         │ Per-document and aggregate cost tracking         │
│                         │ Budget alerts, cost breakdown by operation       │
├─────────────────────────┼───────────────────────────────────────────────────┤
│ Chunk Explorer          │ Browse individual chunks with entity highlights  │
│                         │ Extraction metadata, cache hit indicators        │
├─────────────────────────┼───────────────────────────────────────────────────┤
│ Entity Provenance       │ Drill down to entity sources                     │
│                         │ Merge history, relationship graph                │
└─────────────────────────┴───────────────────────────────────────────────────┘
```

### 7.4 New Components Summary

| Component                | Category | Priority | Lines |
| ------------------------ | -------- | -------- | ----- |
| `IngestionProgressPanel` | Progress | P0       | ~300  |
| `StageIndicator`         | Progress | P0       | ~200  |
| `CostBadge`              | Cost     | P0       | ~80   |
| `CostBreakdownChart`     | Cost     | P1       | ~150  |
| `ChunkExplorer`          | Lineage  | P0       | ~250  |
| `LineageGraph`           | Lineage  | P1       | ~400  |
| `EntityProvenance`       | Lineage  | P1       | ~200  |
| `WebSocketStatus`        | Shared   | P0       | ~50   |

### 7.5 WebUI Implementation Phases

```
Phase W1 (Week 6-7): Foundation
  ├── WebSocket client implementation
  ├── Ingestion store (Zustand)
  ├── Updated TypeScript types
  └── WebSocketProvider

Phase W2 (Week 7-8): Progress Components
  ├── IngestionProgressPanel
  ├── StageIndicator
  ├── BatchProgressCard updates
  └── DocumentManager updates

Phase W3 (Week 8-9): Lineage Visualization
  ├── ChunkExplorer
  ├── LineageTreeView (update)
  ├── LineageGraphView (new)
  └── EntityProvenance

Phase W4 (Week 9-10): Cost Monitoring
  ├── CostBadge
  ├── CostDashboard page
  ├── CostBreakdownChart
  └── BudgetIndicator
```

---

## 8. Layout Architecture Verification (v2.1)

> **Added 2024-12-28**: Deep reflection on UI layout patterns, container behavior, and integration roadblocks.

### 8.1 Layout Architecture Pattern

The EdgeQuake WebUI follows a **3-tier layout architecture** that is compatible with the planned ingestion UI:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ TIER 1: APP SHELL                                                           │
│ ┌─────────┬─────────────────────────────────────────────────────────────┐  │
│ │ SIDEBAR │ HEADER (fixed h-12) + BREADCRUMB (fixed py-2)               │  │
│ │ (fixed  │ ┌─────────────────────────────────────────────────────────┐ │  │
│ │  w-56)  │ │ MAIN CONTENT (flex-1 min-h-0 overflow-hidden)          │ │  │
│ │         │ │   → Each page controls its own scrolling                │ │  │
│ │         │ └─────────────────────────────────────────────────────────┘ │  │
│ └─────────┴─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key Pattern**: `min-h-0 overflow-hidden` on `<main>` allows each page to manage its own scrolling.

### 8.2 Container Behavior Matrix

| Container Type | CSS Pattern                    | Use Case                                 |
| -------------- | ------------------------------ | ---------------------------------------- |
| **FIXED**      | `shrink-0`                     | Always visible headers, footers, filters |
| **ATTACHED**   | `shrink-0` + conditional       | Progress panels, upload zones            |
| **EXPANDABLE** | `w-[Npx]` + collapse           | Side panels, modals                      |
| **SCROLLABLE** | `flex-1 min-h-0 overflow-auto` | Tables, lists, content areas             |

### 8.3 Roadblocks & Mitigation

| ID        | Roadblock              | Risk | Mitigation                       |
| --------- | ---------------------- | :--: | -------------------------------- |
| RB-UI-001 | WebSocket Provider     | LOW  | Add to AppProviders chain        |
| RB-UI-002 | Progress Fixed Zone    | LOW  | Follow BatchProgressCard pattern |
| RB-UI-003 | Panel Content Overflow | MED  | Use tabs with independent scroll |
| RB-UI-004 | LineageGraph Viewport  | MED  | Create two variants (full/panel) |
| RB-UI-005 | Mobile Responsive      | MED  | Use Sheet/Drawer patterns        |
| RB-UI-006 | Animation Performance  | LOW  | React.memo, useMemo patterns     |
| RB-UI-007 | State Complexity       | LOW  | Add Zustand stores               |
| RB-UI-008 | Table Crowding         | LOW  | Responsive-hidden columns        |

### 8.4 Accessibility Enhancements Required

| Requirement    | Current | Action                     |
| -------------- | ------- | -------------------------- |
| Touch Targets  | 32px    | Increase to 44px min       |
| Reduced Motion | Missing | Add prefers-reduced-motion |
| Screen Reader  | Partial | Add ARIA to new components |
| Keyboard Nav   | Good    | Extend to new components   |

### 8.5 Verification Result

**✅ VERIFIED**: The WebUI specification is **fully compatible** with the existing codebase architecture.

| Aspect            | Status | Notes                                            |
| ----------------- | :----: | ------------------------------------------------ |
| Layout Patterns   |   ✅   | Uses established flex patterns                   |
| Component Library |   ✅   | shadcn/ui, Tailwind, Radix                       |
| State Management  |   ✅   | Zustand pattern established                      |
| API Integration   |   ✅   | React Query pattern established                  |
| Responsive Design |   ✅   | Mobile patterns exist                            |
| Design Tokens     |   ✅   | design-tokens.css established                    |
| Accessibility     |   ⚠️   | Needs touch target & reduced-motion enhancements |

> **See**: [scratchpad.md - Session 4](scratchpad.md) for detailed container behavior matrix and code-level analysis.

---

## 9. Next Steps

### 9.1 Immediate Actions

1. **Review and approve** this design plan
2. **Create feature branch** `feat/sota-ingestion-pipeline`
3. **Begin Phase 1** implementation
4. **Set up CI** for new test coverage requirements

### 9.2 Implementation Checklist

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

Phase W1: WebUI Foundation
- [ ] Create WebSocket client (progress-websocket.ts)
- [ ] Create Zustand ingestion store
- [ ] Add new TypeScript types
- [ ] Create WebSocketProvider
- [ ] Add WebSocketStatus component

Phase W2: Progress Components
- [ ] Create IngestionProgressPanel
- [ ] Create StageIndicator
- [ ] Update BatchProgressCard
- [ ] Update DocumentManager with cost column

Phase W3: Lineage Visualization
- [ ] Create ChunkExplorer
- [ ] Update LineageTree (interactive)
- [ ] Create LineageGraphView (React Flow)
- [ ] Create EntityProvenance panel

Phase W4: Cost Monitoring
- [ ] Create CostBadge component
- [ ] Create CostDashboard page
- [ ] Create CostBreakdownChart
- [ ] Create BudgetIndicator
- [ ] Add export/reporting
```

### 9.3 Success Metrics

| Metric               | Target | Validation        |
| -------------------- | ------ | ----------------- |
| Line number accuracy | 100%   | Unit tests        |
| Parallel speedup     | 3-4x   | Benchmarks        |
| Cache hit rate       | >50%   | Integration tests |
| Cost reduction       | 30%    | Real LLM tests    |
| Test coverage        | >80%   | CI pipeline       |

---

## Appendix A: Document Index

| Document                                                           | Purpose                                 | Lines |
| ------------------------------------------------------------------ | --------------------------------------- | ----- |
| [01-architecture.md](01-architecture.md)                           | System architecture with ASCII diagrams | ~400  |
| [02-comparison.md](02-comparison.md)                               | Rust vs Python feature comparison       | ~300  |
| [03-data-models.md](03-data-models.md)                             | Complete data model specifications      | ~500  |
| [04-api-contracts.md](04-api-contracts.md)                         | API endpoint definitions                | ~400  |
| [05-implementation-plan.md](05-implementation-plan.md)             | Phased implementation roadmap           | ~600  |
| [06-testing-strategy.md](06-testing-strategy.md)                   | Test plans and strategies               | ~500  |
| [plan.md](plan.md)                                                 | This document - master plan             | ~600  |
| [10-webui-spec-architecture.md](10-webui-spec-architecture.md)     | WebUI architecture overview             | ~350  |
| [11-webui-screen-flows.md](11-webui-screen-flows.md)               | WebUI wireframes and screen flows       | ~450  |
| [12-webui-api-integration.md](12-webui-api-integration.md)         | WebUI API integration layer             | ~500  |
| [13-webui-components.md](13-webui-components.md)                   | WebUI component specifications          | ~400  |
| [14-webui-websocket-progress.md](14-webui-websocket-progress.md)   | WebSocket real-time progress            | ~450  |
| [15-webui-lineage-viz.md](15-webui-lineage-viz.md)                 | Lineage visualization design            | ~500  |
| [16-webui-cost-monitoring.md](16-webui-cost-monitoring.md)         | Cost monitoring UI                      | ~400  |
| [17-webui-implementation-plan.md](17-webui-implementation-plan.md) | WebUI implementation roadmap            | ~500  |

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
