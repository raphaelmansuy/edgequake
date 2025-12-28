# Document Cross-Reference Index

> Document ID: XREF-001
> Version: 1.0
> Created: 2024-12-28

## Table of Contents

1. [Document Inventory](#1-document-inventory)
2. [Cross-Reference Matrix](#2-cross-reference-matrix)
3. [Topic Index](#3-topic-index)
4. [Requirement Traceability](#4-requirement-traceability)
5. [File Reference Index](#5-file-reference-index)

---

## 1. Document Inventory

### 1.1 Planning Documents

| ID | Document | Purpose | Lines | Dependencies |
|----|----------|---------|-------|--------------|
| DOC-00 | [plan.md](plan.md) | Master plan & executive summary | ~400 | All documents |
| DOC-01 | [01-architecture.md](01-architecture.md) | System architecture diagrams | ~400 | None |
| DOC-02 | [02-comparison.md](02-comparison.md) | Rust vs Python comparison | ~300 | DOC-01 |
| DOC-03 | [03-data-models.md](03-data-models.md) | Data model specifications | ~500 | DOC-01, DOC-02 |
| DOC-04 | [04-api-contracts.md](04-api-contracts.md) | API endpoint definitions | ~400 | DOC-03 |
| DOC-05 | [05-implementation-plan.md](05-implementation-plan.md) | Implementation roadmap | ~600 | DOC-01 to DOC-04 |
| DOC-06 | [06-testing-strategy.md](06-testing-strategy.md) | Test plans | ~500 | DOC-03, DOC-05 |
| DOC-07 | [07-prompt-comparison.md](07-prompt-comparison.md) | LLM prompt analysis | ~600 | DOC-02 |
| DOC-08 | [08-documentation-crosscheck.md](08-documentation-crosscheck.md) | Documentation vs code validation | ~300 | All |
| DOC-09 | [09-cross-reference.md](09-cross-reference.md) | This document | ~200 | All |
| DOC-10 | [scratchpad.md](scratchpad.md) | Working notes | ~200 | None |

### 1.2 Document Dependency Graph

```
                                    ┌──────────────┐
                                    │   plan.md    │
                                    │   (DOC-00)   │
                                    └──────┬───────┘
                                           │
              ┌────────────────────────────┼────────────────────────────┐
              │                            │                            │
              ▼                            ▼                            ▼
   ┌──────────────────┐       ┌──────────────────┐        ┌──────────────────┐
   │ 01-architecture  │       │ 02-comparison    │        │ 07-prompts       │
   │    (DOC-01)      │       │    (DOC-02)      │        │    (DOC-07)      │
   └────────┬─────────┘       └────────┬─────────┘        └──────────────────┘
            │                          │
            ▼                          ▼
   ┌──────────────────┐       ┌──────────────────┐
   │ 03-data-models   │◄──────│ 02-comparison    │
   │    (DOC-03)      │       │    (DOC-02)      │
   └────────┬─────────┘       └──────────────────┘
            │
            ▼
   ┌──────────────────┐
   │ 04-api-contracts │
   │    (DOC-04)      │
   └────────┬─────────┘
            │
            ▼
   ┌──────────────────┐       ┌──────────────────┐
   │ 05-implementation│──────▶│ 06-testing       │
   │    (DOC-05)      │       │    (DOC-06)      │
   └────────┬─────────┘       └──────────────────┘
            │
            ▼
   ┌──────────────────┐
   │ 08-crosscheck    │
   │    (DOC-08)      │
   └──────────────────┘
```

---

## 2. Cross-Reference Matrix

### 2.1 Document-to-Document References

| From ↓ / To → | DOC-01 | DOC-02 | DOC-03 | DOC-04 | DOC-05 | DOC-06 | DOC-07 | DOC-08 |
|---------------|--------|--------|--------|--------|--------|--------|--------|--------|
| **DOC-01** Architecture | - | ✓ | ✓ | | ✓ | | | |
| **DOC-02** Comparison | ✓ | - | ✓ | | ✓ | | ✓ | |
| **DOC-03** Data Models | ✓ | ✓ | - | ✓ | ✓ | ✓ | | ✓ |
| **DOC-04** API Contracts | | | ✓ | - | ✓ | ✓ | | ✓ |
| **DOC-05** Implementation | ✓ | ✓ | ✓ | ✓ | - | ✓ | | ✓ |
| **DOC-06** Testing | | | ✓ | | ✓ | - | | |
| **DOC-07** Prompts | | ✓ | | | | | - | |
| **DOC-08** Crosscheck | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - |

### 2.2 Key Concept Locations

| Concept | Primary Doc | Related Docs |
|---------|-------------|--------------|
| TextChunk | DOC-03 §2.2 | DOC-05 §3.2, DOC-08 §2.1 |
| Entity | DOC-03 §2.3 | DOC-01 §3, DOC-08 §2.2 |
| Relationship | DOC-03 §2.4 | DOC-01 §3 |
| Pipeline | DOC-01 §2 | DOC-05 §3, DOC-06 §3.1 |
| MapReduce | DOC-05 §4.2 | DOC-02 §3, DOC-08 §6.3 |
| Gleaning | DOC-02 §3 | DOC-07 §3 |
| Lineage | DOC-03 §4 | DOC-04 §5, DOC-05 §6 |
| Progress | DOC-03 §6 | DOC-04 §3, DOC-05 §5 |
| Cost | DOC-03 §5 | DOC-04 §5, DOC-05 §5 |
| Prompts | DOC-07 | DOC-02 §3 |

---

## 3. Topic Index

### 3.1 Architecture Topics

| Topic | Documents | Sections |
|-------|-----------|----------|
| Pipeline Flow | DOC-01, DOC-05 | 01:§2, 05:§3 |
| Crate Structure | DOC-01, DOC-08 | 01:§4, 08:§5.2 |
| Component Diagram | DOC-01 | 01:§2.2 |
| Data Flow | DOC-01, DOC-03 | 01:§2.1, 03:§1 |
| Multi-tenancy | DOC-01, DOC-03 | 01:§5, 03:§2.1 |

### 3.2 Data Model Topics

| Topic | Documents | Sections |
|-------|-----------|----------|
| Document Model | DOC-03 | 03:§2.1 (DM-001) |
| TextChunk Model | DOC-03, DOC-08 | 03:§2.2 (DM-002), 08:§2.1 |
| Entity Model | DOC-03, DOC-08 | 03:§2.3 (DM-003), 08:§2.2 |
| Relationship Model | DOC-03 | 03:§2.4 |
| Lineage Models | DOC-03 | 03:§4 (LM-001 to LM-003) |
| Cost Models | DOC-03 | 03:§5 (CM-001, CM-002) |
| Progress Models | DOC-03 | 03:§6 (SM-001, SM-002) |

### 3.3 API Topics

| Topic | Documents | Sections |
|-------|-----------|----------|
| Document Upload | DOC-04 | 04:§2.1 |
| Progress Tracking | DOC-04 | 04:§3 |
| Lineage Endpoints | DOC-04 | 04:§4 |
| Cost Endpoints | DOC-04 | 04:§5 |
| Document Management | DOC-04 | 04:§6 |
| WebSocket Events | DOC-04 | 04:§7 |
| Error Codes | DOC-04 | 04:§8 |

### 3.4 Implementation Topics

| Topic | Documents | Sections |
|-------|-----------|----------|
| Phase 1: Core | DOC-05 | 05:§3 |
| Phase 2: MapReduce | DOC-05 | 05:§4 |
| Phase 3: Progress | DOC-05 | 05:§5 |
| Phase 4: Lineage | DOC-05 | 05:§6 |
| Phase 5: API | DOC-05 | 05:§7 |
| File Changes | DOC-05 | 05:§8 |
| Migration | DOC-05 | 05:§9 |

### 3.5 Testing Topics

| Topic | Documents | Sections |
|-------|-----------|----------|
| Unit Tests | DOC-06 | 06:§2 |
| Integration Tests | DOC-06 | 06:§3 |
| E2E Tests | DOC-06 | 06:§4 |
| Performance Tests | DOC-06 | 06:§5 |
| Test Data | DOC-06 | 06:§6 |
| CI Pipeline | DOC-06 | 06:§7 |
| Quality Gates | DOC-06 | 06:§8 |

### 3.6 Prompt Topics

| Topic | Documents | Sections |
|-------|-----------|----------|
| Entity Extraction | DOC-07 | 07:§2 |
| Gleaning | DOC-07 | 07:§3 |
| Summarization | DOC-07 | 07:§4 |
| Keywords | DOC-07 | 07:§5 |
| RAG Response | DOC-07 | 07:§6 |
| Gap Analysis | DOC-07 | 07:§7 |

---

## 4. Requirement Traceability

### 4.1 Specification Requirements to Documents

**Source:** `specs/19-ingestion-pipeline.md`

| Req ID | Requirement | Primary Doc | Implementation |
|--------|-------------|-------------|----------------|
| F001 | Document ingestion endpoint | DOC-04 §2.1 | DOC-05 §7.1 |
| F002 | Chunk-level lineage with line numbers | DOC-03 §4 | DOC-05 §3.1 |
| F003 | MapReduce description summarization | DOC-02 §3 | DOC-05 §4.2 |
| F004 | Parallel chunk processing | DOC-01 §3 | DOC-05 §3.3 |
| F005 | LLM response caching | DOC-02 §3 | DOC-05 §4.3 |
| F006 | Real-time progress tracking | DOC-03 §6 | DOC-05 §5.3 |
| F007 | Cost tracking per operation | DOC-03 §5 | DOC-05 §5.4 |
| F008 | Document suppression | DOC-04 §6 | DOC-05 §6.4 |
| F009 | Entity CRUD operations | DOC-04 §6 | DOC-05 §6.5 |
| F010 | Gleaning multi-pass extraction | DOC-07 §3 | DOC-08 §6.3 |
| F011 | Multi-tenant isolation | DOC-01 §5 | DOC-08 §5.2 |
| F012 | WebSocket progress events | DOC-04 §7 | DOC-05 §7.4 |
| R001 | Line number tracking | DOC-03 §2.2 | DOC-05 §3.2 |

### 4.2 Data Model Traceability

| Model ID | Definition | Usage | Tests |
|----------|------------|-------|-------|
| DM-001 | DOC-03 §2.1 | DOC-04 §2, DOC-05 §6 | DOC-06 §2.1 |
| DM-002 | DOC-03 §2.2 | DOC-05 §3, DOC-08 §2.1 | DOC-06 §2.1 |
| DM-003 | DOC-03 §2.3 | DOC-05 §3, DOC-08 §2.2 | DOC-06 §2.2 |
| PM-001 | DOC-03 §3.1 | DOC-05 §3 | DOC-06 §2.1 |
| PM-002 | DOC-03 §3.2 | DOC-05 §3 | DOC-06 §2.2 |
| LM-001 | DOC-03 §4.1 | DOC-04 §4, DOC-05 §6 | DOC-06 §3.2 |
| LM-002 | DOC-03 §4.2 | DOC-04 §4, DOC-05 §6 | DOC-06 §3.2 |
| LM-003 | DOC-03 §4.3 | DOC-04 §4, DOC-05 §6 | DOC-06 §3.2 |
| CM-001 | DOC-03 §5.1 | DOC-04 §5, DOC-05 §5 | DOC-06 §2.5 |
| CM-002 | DOC-03 §5.2 | DOC-04 §5, DOC-05 §5 | DOC-06 §2.5 |
| SM-001 | DOC-03 §6.1 | DOC-04 §3, DOC-05 §5 | DOC-06 §3.1 |
| SM-002 | DOC-03 §6.2 | DOC-04 §7, DOC-05 §5 | DOC-06 §3.1 |

---

## 5. File Reference Index

### 5.1 Rust Source Files Referenced

| File | Referenced In | Topic |
|------|---------------|-------|
| `edgequake-pipeline/src/chunker.rs` | DOC-05 §3.2, DOC-08 §2.1 | TextChunk, ChunkerConfig |
| `edgequake-pipeline/src/extractor.rs` | DOC-05 §3.4, DOC-07 §2.2, DOC-08 §2.2 | Entity extraction, prompts |
| `edgequake-pipeline/src/merger.rs` | DOC-05 §4.7, DOC-01 §2 | Knowledge graph merging |
| `edgequake-pipeline/src/summarizer.rs` | DOC-05 §4.2, DOC-07 §4.2 | MapReduce summarization |
| `edgequake-pipeline/src/pipeline.rs` | DOC-05 §3.3, DOC-01 §2 | Pipeline orchestration |
| `edgequake-core/src/orchestrator.rs` | DOC-01 §2, DOC-08 §5.1 | EdgeQuake main class |
| `edgequake-core/src/tenant_manager.rs` | DOC-01 §5, DOC-08 §5.1 | Multi-tenancy |
| `edgequake-query/src/keywords.rs` | DOC-07 §5.2 | Keyword extraction |
| `edgequake-query/src/engine.rs` | DOC-07 §6.2 | RAG response generation |
| `edgequake-llm/src/cache.rs` | DOC-05 §4.3, DOC-08 §6.3 | LLM caching |
| `edgequake-api/src/handlers/*.rs` | DOC-04, DOC-08 §3.1 | API endpoints |

### 5.2 Python Source Files Referenced

| File | Referenced In | Topic |
|------|---------------|-------|
| `lightrag/prompt.py` | DOC-07 §2.1, §3.1, §4.1, §5.1, §6.1 | All LightRAG prompts |
| `lightrag/operate.py` | DOC-02 §3, DOC-08 §2 | Operations, MapReduce |
| `lightrag/base.py` | DOC-02 §2 | Storage interfaces |

### 5.3 New Files to Create (per DOC-05)

| File | Phase | Purpose | Reference |
|------|-------|---------|-----------|
| `edgequake-pipeline/src/cache.rs` | 2 | Pipeline-specific caching | DOC-05 §4.3 |
| `edgequake-core/src/progress.rs` | 3 | Progress tracking | DOC-05 §5.3 |
| `edgequake-core/src/cost.rs` | 3 | Cost calculation | DOC-05 §5.4 |
| `edgequake-core/src/types/lineage.rs` | 4 | Lineage types | DOC-05 §6.1 |
| `edgequake-storage/src/adapters/lineage.rs` | 4 | Lineage storage | DOC-05 §6.2 |
| `edgequake-api/src/ws.rs` | 5 | WebSocket handler | DOC-05 §7.4 |

---

## 6. Quick Navigation

### By Implementation Phase

| Phase | Planning | Models | API | Tests |
|-------|----------|--------|-----|-------|
| Phase 1 | [DOC-05 §3](05-implementation-plan.md) | [DOC-03 §2.2](03-data-models.md) | - | [DOC-06 §2.1](06-testing-strategy.md) |
| Phase 2 | [DOC-05 §4](05-implementation-plan.md) | [DOC-03 §3](03-data-models.md) | - | [DOC-06 §2.3](06-testing-strategy.md) |
| Phase 3 | [DOC-05 §5](05-implementation-plan.md) | [DOC-03 §5,6](03-data-models.md) | [DOC-04 §3](04-api-contracts.md) | [DOC-06 §3.1](06-testing-strategy.md) |
| Phase 4 | [DOC-05 §6](05-implementation-plan.md) | [DOC-03 §4](03-data-models.md) | [DOC-04 §4](04-api-contracts.md) | [DOC-06 §3.2](06-testing-strategy.md) |
| Phase 5 | [DOC-05 §7](05-implementation-plan.md) | - | [DOC-04 §2-7](04-api-contracts.md) | [DOC-06 §4](06-testing-strategy.md) |

### By Feature

| Feature | Design | Implementation | Tests | Code |
|---------|--------|----------------|-------|------|
| Line Numbers | DOC-03 §2.2 | DOC-05 §3.1-2 | DOC-06 §2.1 | chunker.rs |
| Parallel | DOC-01 §3 | DOC-05 §3.3 | DOC-06 §3.1 | pipeline.rs |
| MapReduce | DOC-02 §3 | DOC-05 §4.1-2 | DOC-06 §2.3 | summarizer.rs ✅ |
| Caching | DOC-02 §3 | DOC-05 §4.3-5 | DOC-06 §2.4 | cache.rs |
| Progress | DOC-03 §6 | DOC-05 §5.3 | DOC-06 §3.1 | progress.rs |
| Cost | DOC-03 §5 | DOC-05 §5.4 | DOC-06 §2.5 | cost.rs |
| Lineage | DOC-03 §4 | DOC-05 §6 | DOC-06 §3.2 | lineage.rs |
| Prompts | DOC-07 | DOC-07 §8 | DOC-06 §2.2 | extractor.rs |

---

## 7. Document Change Log

| Date | Document | Change | Author |
|------|----------|--------|--------|
| 2024-12-28 | DOC-01 | Initial creation | Claude |
| 2024-12-28 | DOC-02 | Initial creation | Claude |
| 2024-12-28 | DOC-03 | Initial creation | Claude |
| 2024-12-28 | DOC-04 | Initial creation | Claude |
| 2024-12-28 | DOC-05 | Initial creation | Claude |
| 2024-12-28 | DOC-06 | Initial creation | Claude |
| 2024-12-28 | DOC-07 | Initial creation - Prompt comparison | Claude |
| 2024-12-28 | DOC-08 | Initial creation - Code crosscheck | Claude |
| 2024-12-28 | DOC-09 | Initial creation - This index | Claude |

---
