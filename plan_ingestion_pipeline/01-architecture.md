# Architecture Overview: SOTA GenAI Ingestion Pipeline

> Document ID: ARCH-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current Architecture](#2-current-architecture)
3. [Target Architecture](#3-target-architecture)
4. [Component Deep Dive](#4-component-deep-dive)
5. [Data Flow](#5-data-flow)
6. [Integration Points](#6-integration-points)

---

## 1. Executive Summary

This document describes the architecture for EdgeQuake's SOTA (State-of-the-Art) GenAI-powered ingestion pipeline. The pipeline transforms unstructured documents into a queryable knowledge graph with full lineage tracking, cost management, and multi-tenant isolation.

**Key Architectural Goals:**
- **R001**: Complete lineage tracking from document to entity
- **R002**: Cost-aware processing with budget controls
- **R003**: Scalable MapReduce-style processing
- **R004**: Real-time progress visibility
- **R005**: Multi-tenant namespace isolation

---

## 2. Current Architecture

### 2.1 High-Level Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CURRENT PIPELINE FLOW                           │
└─────────────────────────────────────────────────────────────────────────┘

┌──────────┐    ┌──────────┐    ┌───────────┐    ┌────────┐    ┌─────────┐
│ Document │───▶│ Chunker  │───▶│ Extractor │───▶│ Merger │───▶│ Storage │
└──────────┘    └──────────┘    └───────────┘    └────────┘    └─────────┘
                     │               │                              │
                     ▼               ▼                              ▼
               ┌──────────┐   ┌───────────┐                  ┌───────────┐
               │TextChunk │   │ Entities  │                  │Graph Store│
               │ + Offset │   │ Relations │                  │Vector DB  │
               └──────────┘   └───────────┘                  │ KV Store  │
                                                             └───────────┘
```

### 2.2 Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CRATE DEPENDENCIES                              │
└─────────────────────────────────────────────────────────────────────────┘

                        ┌─────────────────┐
                        │  edgequake-api  │
                        │   (REST API)    │
                        └────────┬────────┘
                                 │
                    ┌────────────┼────────────┐
                    │            │            │
                    ▼            ▼            ▼
           ┌────────────┐ ┌───────────┐ ┌──────────┐
           │edgequake-  │ │edgequake- │ │edgequake-│
           │   core     │ │  tasks    │ │   auth   │
           │(Orchestr.) │ │(Background)│ │  (Auth)  │
           └─────┬──────┘ └─────┬─────┘ └──────────┘
                 │              │
    ┌────────────┼──────────────┼────────────┐
    │            │              │            │
    ▼            ▼              ▼            ▼
┌────────┐ ┌──────────┐ ┌───────────┐ ┌──────────┐
│edgequake│ │edgequake-│ │ edgequake-│ │edgequake-│
│  -llm  │ │ pipeline │ │   query   │ │ storage  │
│(LLM)   │ │(Pipeline)│ │ (Query)   │ │(Storage) │
└────────┘ └──────────┘ └───────────┘ └──────────┘
```

### 2.3 Current Component Details

| Component | Responsibility | Crate |
|-----------|---------------|-------|
| Pipeline | Orchestrates doc→chunk→extract→merge | edgequake-pipeline |
| Chunker | Splits text into overlapping chunks | edgequake-pipeline |
| Extractor | LLM-based entity/relation extraction | edgequake-pipeline |
| GleaningExtractor | Re-extraction for missed entities | edgequake-pipeline |
| Merger | Deduplication, description merging | edgequake-pipeline |
| Summarizer | Description condensation | edgequake-pipeline |
| Orchestrator | High-level RAG coordination | edgequake-core |
| TenantManager | Multi-tenant instance management | edgequake-core |

---

## 3. Target Architecture

### 3.1 Enhanced Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    TARGET SOTA PIPELINE ARCHITECTURE                    │
└─────────────────────────────────────────────────────────────────────────┘

   ┌───────────────────── DOCUMENT INGESTION PIPELINE ─────────────────────┐
   │                                                                        │
   │  ┌────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌───────┐ │
   │  │Document│──▶│Pre-Proc. │──▶│ Chunker  │──▶│  MAP     │──▶│REDUCE │ │
   │  │ Upload │   │(validate,│   │(w/line # │   │(parallel │   │(merge │ │
   │  │        │   │ detect)  │   │ tracking)│   │ extract) │   │dedupe)│ │
   │  └────────┘   └──────────┘   └──────────┘   └──────────┘   └───────┘ │
   │      │             │              │              │             │      │
   │      │             │              │              │             │      │
   │      ▼             ▼              ▼              ▼             ▼      │
   │  ┌─────────────────────────────────────────────────────────────────┐ │
   │  │                    LINEAGE TRACKER (R001)                        │ │
   │  │   doc_id → [chunk_ids] → [entity_ids] → [relationship_ids]     │ │
   │  └─────────────────────────────────────────────────────────────────┘ │
   │      │             │              │              │             │      │
   │      ▼             ▼              ▼              ▼             ▼      │
   │  ┌─────────────────────────────────────────────────────────────────┐ │
   │  │                    COST TRACKER (R002)                           │ │
   │  │   input_tokens, output_tokens, embedding_tokens, cost_usd       │ │
   │  └─────────────────────────────────────────────────────────────────┘ │
   │      │             │              │              │             │      │
   │      ▼             ▼              ▼              ▼             ▼      │
   │  ┌─────────────────────────────────────────────────────────────────┐ │
   │  │                  PROGRESS REPORTER (R004)                        │ │
   │  │   stage, completion_pct, current_step, eta, errors              │ │
   │  └─────────────────────────────────────────────────────────────────┘ │
   │                                                                        │
   └────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
   ┌────────────────────── STORAGE LAYER ─────────────────────────────────┐
   │                                                                        │
   │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌──────────┐ │
   │  │ Graph Store │   │ Vector DB   │   │  KV Store   │   │ Lineage  │ │
   │  │  (AGE/Neo4j)│   │ (pgvector)  │   │(Documents,  │   │  Store   │ │
   │  │  Entities,  │   │  Entities,  │   │  Chunks,    │   │(tracking)│ │
   │  │  Relations  │   │  Relations, │   │  Cache)     │   │          │ │
   │  │             │   │  Chunks     │   │             │   │          │ │
   │  └─────────────┘   └─────────────┘   └─────────────┘   └──────────┘ │
   │                                                                        │
   └────────────────────────────────────────────────────────────────────────┘
```

### 3.2 MapReduce Processing Pattern (R003)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      MAP-REDUCE EXTRACTION FLOW                         │
└─────────────────────────────────────────────────────────────────────────┘

                     ┌─────────────────────────────┐
                     │      Large Document         │
                     │     (> 50 chunks)           │
                     └─────────────┬───────────────┘
                                   │
                     ┌─────────────▼───────────────┐
                     │         SPLIT               │
                     │   (into N chunk batches)    │
                     └─────────────┬───────────────┘
                                   │
           ┌───────────────────────┼───────────────────────┐
           │                       │                       │
           ▼                       ▼                       ▼
    ┌──────────────┐       ┌──────────────┐       ┌──────────────┐
    │   MAP #1     │       │   MAP #2     │       │   MAP #N     │
    │  Extract     │       │  Extract     │       │  Extract     │
    │  Entities    │       │  Entities    │       │  Entities    │
    │  + Rels      │       │  + Rels      │       │  + Rels      │
    └──────┬───────┘       └──────┬───────┘       └──────┬───────┘
           │                       │                       │
           │      ┌────────────────┼────────────────┐      │
           │      │                │                │      │
           ▼      ▼                ▼                ▼      ▼
    ┌─────────────────────────────────────────────────────────────┐
    │                         REDUCE                               │
    │  1. Collect all extracted entities and relationships        │
    │  2. Normalize entity names (uppercase, trim)                │
    │  3. Group by entity/relationship key                        │
    │  4. Merge descriptions (map-reduce if > token limit)        │
    │  5. Calculate final weights and importance                  │
    └─────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
    ┌─────────────────────────────────────────────────────────────┐
    │                        OUTPUT                                │
    │  Deduplicated entities and relationships with merged        │
    │  descriptions, aggregated source references, and            │
    │  combined embeddings                                        │
    └─────────────────────────────────────────────────────────────┘
```

### 3.3 Multi-Tenant Namespace Isolation (R005)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     MULTI-TENANT ARCHITECTURE                           │
└─────────────────────────────────────────────────────────────────────────┘

                            ┌─────────────┐
                            │  API Layer  │
                            │ (Auth/RBAC) │
                            └──────┬──────┘
                                   │
         ┌─────────────────────────┼─────────────────────────┐
         │                         │                         │
         ▼                         ▼                         ▼
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│   Tenant: A     │      │   Tenant: B     │      │   Tenant: C     │
│   ───────────   │      │   ───────────   │      │   ───────────   │
│ ┌─────────────┐ │      │ ┌─────────────┐ │      │ ┌─────────────┐ │
│ │ Workspace 1 │ │      │ │ Workspace 1 │ │      │ │ Workspace 1 │ │
│ │  - Graph    │ │      │ │  - Graph    │ │      │ │  - Graph    │ │
│ │  - Vectors  │ │      │ │  - Vectors  │ │      │ │  - Vectors  │ │
│ │  - Docs     │ │      │ │  - Docs     │ │      │ │  - Docs     │ │
│ └─────────────┘ │      │ └─────────────┘ │      │ └─────────────┘ │
│ ┌─────────────┐ │      │ ┌─────────────┐ │      │                 │
│ │ Workspace 2 │ │      │ │ Workspace 2 │ │      │                 │
│ │  - Graph    │ │      │ │  - Graph    │ │      │                 │
│ │  - Vectors  │ │      │ │  - Vectors  │ │      │                 │
│ │  - Docs     │ │      │ │  - Docs     │ │      │                 │
│ └─────────────┘ │      │ └─────────────┘ │      │                 │
└─────────────────┘      └─────────────────┘      └─────────────────┘

         │                         │                         │
         └─────────────────────────┼─────────────────────────┘
                                   │
                                   ▼
                    ┌─────────────────────────────┐
                    │     Shared Infrastructure   │
                    │  - PostgreSQL (partitioned) │
                    │  - LLM Providers (pooled)   │
                    │  - Rate Limiters            │
                    └─────────────────────────────┘
```

---

## 4. Component Deep Dive

### 4.1 Enhanced Chunker (F001)

```rust
// NEW: TextChunk with line number tracking
pub struct TextChunk {
    pub id: String,
    pub content: String,
    pub index: usize,
    // Character offsets
    pub start_offset: usize,
    pub end_offset: usize,
    // NEW: Line number tracking (R001)
    pub start_line: usize,
    pub end_line: usize,
    // Token info
    pub token_count: usize,
    // Source reference
    pub document_id: String,
    pub document_name: Option<String>,
    // Embedding
    pub embedding: Option<Vec<f32>>,
}
```

### 4.2 Lineage Tracker (F002)

```rust
// NEW: Complete lineage tracking
pub struct DocumentLineage {
    pub document_id: String,
    pub chunks: Vec<ChunkLineage>,
    pub created_at: DateTime<Utc>,
    pub ingestion_config: IngestionConfig,
}

pub struct ChunkLineage {
    pub chunk_id: String,
    pub start_line: usize,
    pub end_line: usize,
    pub entities: Vec<String>,         // Entity IDs
    pub relationships: Vec<String>,     // Relationship IDs
    pub extraction_metadata: ExtractionMetadata,
}

pub struct ExtractionMetadata {
    pub llm_model: String,
    pub gleaning_iterations: usize,
    pub extraction_time_ms: u64,
    pub tokens_used: TokenUsage,
}
```

### 4.3 Cost Tracker (F003)

```rust
// NEW: Comprehensive cost tracking
pub struct IngestionCost {
    pub document_id: String,
    pub total_cost_usd: f64,
    pub breakdown: CostBreakdown,
    pub created_at: DateTime<Utc>,
}

pub struct CostBreakdown {
    // LLM extraction costs
    pub extraction_input_tokens: usize,
    pub extraction_output_tokens: usize,
    pub extraction_cost_usd: f64,
    
    // Embedding costs
    pub embedding_tokens: usize,
    pub embedding_cost_usd: f64,
    
    // Summarization costs (if used)
    pub summarization_input_tokens: usize,
    pub summarization_output_tokens: usize,
    pub summarization_cost_usd: f64,
    
    // Gleaning costs
    pub gleaning_input_tokens: usize,
    pub gleaning_output_tokens: usize,
    pub gleaning_cost_usd: f64,
    
    // Model info
    pub extraction_model: String,
    pub embedding_model: String,
}
```

### 4.4 Progress Reporter (F004)

```rust
// NEW: Real-time progress tracking
pub struct IngestionProgress {
    pub track_id: String,
    pub document_id: String,
    pub status: IngestionStatus,
    pub stages: Vec<StageProgress>,
    pub current_stage: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub errors: Vec<IngestionError>,
}

pub enum IngestionStatus {
    Pending,
    Preprocessing,
    Chunking,
    Extracting,
    Merging,
    Embedding,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

pub struct StageProgress {
    pub stage: IngestionStatus,
    pub total_items: usize,
    pub completed_items: usize,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

---

## 5. Data Flow

### 5.1 Complete Ingestion Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    COMPLETE DATA FLOW DIAGRAM                           │
└─────────────────────────────────────────────────────────────────────────┘

Step 1: Document Upload
═══════════════════════
  ┌────────────────┐
  │  API Request   │ ──────────────────────────────────────┐
  │ POST /documents│                                       │
  │  {content,     │                                       ▼
  │   filename,    │                            ┌─────────────────┐
  │   workspace_id}│                            │  PreProcessor   │
  └────────────────┘                            │  - Validate     │
                                                │  - Detect type  │
                                                │  - Extract text │
                                                └────────┬────────┘
                                                         │
Step 2: Preprocessing                                    ▼
═══════════════════════                        ┌─────────────────┐
  Creates:                                     │  Document       │
  - document_id                                │  - id           │
  - track_id for progress                      │  - content      │
  - workspace context                          │  - filename     │
                                               │  - workspace_id │
                                               │  - created_at   │
                                               └────────┬────────┘
                                                        │
Step 3: Chunking                                        ▼
════════════════════                           ┌─────────────────┐
  Creates:                                     │    Chunker      │
  - TextChunk[] with line numbers              │  - split text   │
  - ChunkLineage entries                       │  - track lines  │
  - Token counts                               │  - add overlap  │
                                               └────────┬────────┘
                                                        │
                            ┌───────────────────────────┼───────────┐
                            │                           │           │
                            ▼                           ▼           ▼
                     ┌───────────┐             ┌───────────┐ ┌───────────┐
                     │  Chunk 0  │             │  Chunk 1  │ │  Chunk N  │
                     │ lines 1-50│             │ lines 45-95│ │lines...   │
                     └─────┬─────┘             └─────┬─────┘ └─────┬─────┘
                           │                         │             │
Step 4: MAP Phase          │                         │             │
══════════════════         │                         │             │
  (Parallel extraction)    ▼                         ▼             ▼
                     ┌───────────┐             ┌───────────┐ ┌───────────┐
                     │ Extractor │             │ Extractor │ │ Extractor │
                     │  + Glean  │             │  + Glean  │ │  + Glean  │
                     └─────┬─────┘             └─────┬─────┘ └─────┬─────┘
                           │                         │             │
                           ▼                         ▼             ▼
                     ┌───────────┐             ┌───────────┐ ┌───────────┐
                     │ Entities: │             │ Entities: │ │ Entities: │
                     │ - JOHN_DOE│             │ - ACME    │ │ - ...     │
                     │ - ACME    │             │ - PROJECT │ │           │
                     │ Relations:│             │ Relations:│ │           │
                     │ - WORKS_AT│             │ - USES    │ │           │
                     └─────┬─────┘             └─────┬─────┘ └─────┬─────┘
                           │                         │             │
Step 5: REDUCE Phase       └─────────────────────────┼─────────────┘
════════════════════                                 │
  (Merge and deduplicate)                            ▼
                                             ┌───────────────┐
                                             │    Reducer    │
                                             │ - Normalize   │
                                             │ - Group by key│
                                             │ - Merge descs │
                                             │ - Aggregate   │
                                             └───────┬───────┘
                                                     │
Step 6: Embedding Generation                         ▼
════════════════════════════                 ┌───────────────┐
  Creates embeddings for:                    │   Embedder    │
  - Chunks                                   │ - Chunk embeds│
  - Entities                                 │ - Entity embeds
  - Relationships                            │ - Rel embeds  │
                                             └───────┬───────┘
                                                     │
Step 7: Storage                                      ▼
═══════════════════                          ┌───────────────┐
  Persists to:                               │    Storage    │
  - GraphStorage: entities, rels             │ - Graph       │
  - VectorStorage: embeddings                │ - Vector      │
  - KVStorage: documents, chunks             │ - KV          │
  - LineageStorage: tracking                 │ - Lineage     │
                                             └───────────────┘
```

---

## 6. Integration Points

### 6.1 LLM Provider Integration

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    LLM PROVIDER INTEGRATION                             │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌─────────────────────────┐
                    │   LLMProviderFactory    │
                    │  (auto-detect from env) │
                    └───────────┬─────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│ OpenAIProvider│      │ OllamaProvider│      │  MockProvider │
│               │      │               │      │  (testing)    │
│ - gpt-4o-mini │      │ - llama3.2    │      │               │
│ - gpt-4       │      │ - mistral     │      │               │
│ - text-embed-3│      │ - nomic-embed │      │               │
└───────────────┘      └───────────────┘      └───────────────┘
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                │
                                ▼
                    ┌─────────────────────────┐
                    │     Rate Limiter        │
                    │  - Per-provider limits  │
                    │  - Token budgets        │
                    │  - Request queuing      │
                    └─────────────────────────┘
```

### 6.2 Storage Backend Integration

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   STORAGE BACKEND INTEGRATION                           │
└─────────────────────────────────────────────────────────────────────────┘

                        ┌─────────────────┐
                        │  StorageFactory │
                        └────────┬────────┘
                                 │
     ┌───────────────────────────┼───────────────────────────┐
     │                           │                           │
     ▼                           ▼                           ▼
┌─────────────┐          ┌─────────────────┐          ┌──────────────┐
│  Memory     │          │   PostgreSQL    │          │  SurrealDB   │
│ (testing)   │          │                 │          │              │
├─────────────┤          ├─────────────────┤          ├──────────────┤
│MemoryGraph  │          │ PostgresGraph   │          │ SurrealGraph │
│MemoryVector │          │ (Apache AGE)    │          │              │
│ MemoryKV    │          │ PostgresVector  │          │ SurrealVector│
│             │          │ (pgvector)      │          │              │
│             │          │ PostgresKV      │          │ SurrealKV    │
└─────────────┘          └─────────────────┘          └──────────────┘
```

### 6.3 Evaluation Suite Integration (Future)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   EVALUATION SUITE INTEGRATION                          │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌─────────────────────────┐
                    │   IngestionPipeline     │
                    └───────────┬─────────────┘
                                │
                                ▼ (metrics export)
                    ┌─────────────────────────┐
                    │   EvaluationExporter    │
                    └───────────┬─────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│    RAGAS      │      │    MLflow     │      │  Custom       │
│               │      │               │      │  Metrics      │
│ - Faithfulness│      │ - Experiments │      │               │
│ - Context Rel.│      │ - Parameters  │      │ - Entity F1   │
│ - Answer Sim. │      │ - Artifacts   │      │ - Relation F1 │
└───────────────┘      └───────────────┘      └───────────────┘
```

---

## Appendix A: Feature Matrix

| Feature ID | Feature Name | Priority | Status |
|------------|--------------|----------|--------|
| F001 | Line number tracking | P0 | To Implement |
| F002 | Full lineage tracking | P0 | To Implement |
| F003 | Cost tracking | P0 | To Implement |
| F004 | Progress reporting | P0 | To Implement |
| F005 | MapReduce extraction | P1 | To Implement |
| F006 | Document suppression | P1 | To Implement |
| F007 | Entity CRUD with cascade | P1 | Partial |
| F008 | Citation retrieval | P1 | To Implement |
| F009 | Multi-LLM provider support | P2 | Implemented |
| F010 | Ontology schema support | P3 | Future |
| F011 | RAGAS integration | P3 | Future |
| F012 | MLflow integration | P3 | Future |

## Appendix B: Requirement Traceability

| Req ID | Requirement | Implementing Features |
|--------|-------------|----------------------|
| R001 | Complete lineage tracking | F001, F002 |
| R002 | Cost-aware processing | F003 |
| R003 | Scalable MapReduce | F005 |
| R004 | Real-time progress | F004 |
| R005 | Multi-tenant isolation | Existing + F002 |

---
