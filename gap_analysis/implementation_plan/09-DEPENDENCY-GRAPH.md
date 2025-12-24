# Dependency Graph & Task Sequencing

**Document ID:** 09-DEPENDENCY-GRAPH  
**Priority:** 🔴 P0 CRITICAL  
**Scope:** All phases  
**Owner:** Project Lead

---

## 📋 Overview

This document visualizes task dependencies and provides optimal sequencing for implementation.

### Cross-References

| Phase   | Document                                                   | Tasks       |
| ------- | ---------------------------------------------------------- | ----------- |
| Phase 1 | [01-PHASE1-QUERY-ENGINE.md](./01-PHASE1-QUERY-ENGINE.md)   | Query modes |
| Phase 1 | [02-PHASE1-MULTI-TENANCY.md](./02-PHASE1-MULTI-TENANCY.md) | Tenant mgmt |
| Phase 2 | [03-PHASE2-CORE-QUALITY.md](./03-PHASE2-CORE-QUALITY.md)   | Quality     |
| Phase 2 | [04-PHASE2-LLM-PROVIDERS.md](./04-PHASE2-LLM-PROVIDERS.md) | Providers   |
| Phase 3 | [05-PHASE3-STORAGE.md](./05-PHASE3-STORAGE.md)             | Storage     |
| Phase 3 | [06-PHASE3-API-FEATURES.md](./06-PHASE3-API-FEATURES.md)   | API         |
| Testing | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md)     | Tests       |
| Risks   | [08-RISK-MITIGATION.md](./08-RISK-MITIGATION.md)           | Risks       |
| Master  | [00-INDEX.md](./00-INDEX.md)                               | Overview    |

---

## 📊 Dependency Graph

```
                    ┌─────────────────────────────────────────────────┐
                    │                    PHASE 1                       │
                    │              P0 CRITICAL BLOCKERS               │
                    └─────────────────────────────────────────────────┘
                                           │
            ┌──────────────────────────────┼──────────────────────────────┐
            │                              │                              │
            ▼                              ▼                              ▼
    ┌───────────────┐             ┌───────────────┐              ┌───────────────┐
    │  TASK-001     │             │  TASK-002     │              │  TASK-003     │
    │ VectorNamespace│            │ Global Query  │              │ TenantRAGMgr  │
    │  Enum         │             │  Mode         │              │               │
    │ 0.5 days      │             │  3 days       │              │  4 days       │
    └───────┬───────┘             └───────┬───────┘              └───────┬───────┘
            │                              │                              │
            │                              │                              │
            ▼                              ▼                              ▼
    ┌───────────────┐             ┌───────────────┐              ┌───────────────┐
    │  TASK-004     │             │  TASK-005     │              │  TASK-006     │
    │ Relationship  │             │ Mix Query     │──────────────│ Tenant        │
    │ Embeddings    │             │ Mode          │              │ Middleware    │
    │ 1 day         │             │ 2 days        │              │ 1 day         │
    └───────┬───────┘             └───────┬───────┘              └───────────────┘
            │                              │
            └──────────────┬───────────────┘
                           │
                    ┌──────▼──────┐
                    │  PHASE 1    │
                    │  COMPLETE   │
                    │  GATE       │
                    └──────┬──────┘
                           │
    ┌─────────────────────────────────────────────────────────────────────┐
    │                           PHASE 2                                    │
    │                     P1 HIGH PRIORITY                                │
    └─────────────────────────────────────────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┬──────────────────┐
        │                  │                  │                  │
        ▼                  ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│  TASK-007     │  │  TASK-008     │  │  TASK-009     │  │  TASK-010     │
│ Deduplication │  │ Keyword       │  │ Token         │  │ Anthropic     │
│               │  │ Extraction    │  │ Budget        │  │ Provider      │
│ 2 days        │  │ 1 day         │  │ 1 day         │  │ 3 days        │
└───────┬───────┘  └───────────────┘  └───────────────┘  └───────┬───────┘
        │                                                         │
        ▼                                                         ▼
┌───────────────┐                                         ┌───────────────┐
│  TASK-011     │                                         │  TASK-012     │
│ Map-Reduce    │                                         │ Rate          │
│ Summarization │                                         │ Limiter       │
│ 2 days        │                                         │ 1 day         │
└───────┬───────┘                                         └───────┬───────┘
        │                                                         │
        ▼                                                         ▼
┌───────────────┐                                         ┌───────────────┐
│  TASK-013     │                                         │  TASK-014     │
│ Jina          │                                         │ LLM           │
│ Reranker      │                                         │ Cache         │
│ 2 days        │                                         │ 2 days        │
└───────┬───────┘                                         └───────────────┘
        │
        └─────────────────────┬───────────────────────────────────┘
                              │
                       ┌──────▼──────┐
                       │  PHASE 2    │
                       │  COMPLETE   │
                       │  GATE       │
                       └──────┬──────┘
                              │
    ┌─────────────────────────────────────────────────────────────────────┐
    │                           PHASE 3                                    │
    │                    P2/P3 EXPANSION                                  │
    └─────────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│  TASK-015     │     │  TASK-016     │     │  TASK-017     │
│ Neo4j         │     │ Qdrant        │     │ Redis         │
│ Storage       │     │ Storage       │     │ KV            │
│ 4 days        │     │ 3 days        │     │ 3 days        │
└───────────────┘     └───────────────┘     └───────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│  TASK-018     │     │  TASK-019     │     │  TASK-020     │
│ Document      │     │ Graph Labels  │     │ Reprocess     │
│ Scan API      │     │ Popular       │     │ Failed        │
│ 2 days        │     │ 1 day         │     │ 2 days        │
└───────────────┘     └───────────────┘     └───────────────┘
                              │
                       ┌──────▼──────┐
                       │  PHASE 3    │
                       │  COMPLETE   │
                       │  GATE       │
                       └──────┬──────┘
                              │
    ┌─────────────────────────────────────────────────────────────────────┐
    │                           PHASE 4                                    │
    │                      VALIDATION                                     │
    └─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌───────────────┐
                    │  TASK-021     │
                    │ Integration   │
                    │ Testing       │
                    │ 3 days        │
                    └───────┬───────┘
                              │
                              ▼
                    ┌───────────────┐
                    │  TASK-022     │
                    │ Performance   │
                    │ Validation    │
                    │ 2 days        │
                    └───────┬───────┘
                              │
                              ▼
                    ┌───────────────┐
                    │  TASK-023     │
                    │ Documentation │
                    │ & Release     │
                    │ 2 days        │
                    └───────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   100% PARITY   │
                    │    ACHIEVED     │
                    └─────────────────┘
```

---

## 📋 Task Details

### Phase 1 Tasks (Weeks 1-3)

| Task ID  | Name                    | Effort | Dependencies | Gap(s)  |
| -------- | ----------------------- | ------ | ------------ | ------- |
| TASK-001 | VectorNamespace Enum    | 0.5d   | None         | GAP-042 |
| TASK-002 | Global Query Mode       | 3d     | None         | GAP-001 |
| TASK-003 | TenantRAGManager        | 4d     | None         | GAP-004 |
| TASK-004 | Relationship Embeddings | 1d     | TASK-001     | GAP-042 |
| TASK-005 | Mix Query Mode          | 2d     | TASK-002     | GAP-002 |
| TASK-006 | Tenant Middleware       | 1d     | TASK-003     | GAP-003 |

### Phase 2 Tasks (Weeks 4-6)

| Task ID  | Name                     | Effort | Dependencies | Gap(s)  |
| -------- | ------------------------ | ------ | ------------ | ------- |
| TASK-007 | Entity Deduplication     | 2d     | Phase 1      | GAP-005 |
| TASK-008 | Keyword Extraction       | 1d     | Phase 1      | GAP-006 |
| TASK-009 | Token Budget             | 1d     | Phase 1      | GAP-019 |
| TASK-010 | Anthropic Provider       | 3d     | Phase 1      | GAP-007 |
| TASK-011 | Map-Reduce Summarization | 2d     | TASK-007     | GAP-018 |
| TASK-012 | Rate Limiter             | 1d     | TASK-010     | GAP-008 |
| TASK-013 | Jina Reranker            | 2d     | TASK-011     | GAP-017 |
| TASK-014 | LLM Cache                | 2d     | TASK-012     | GAP-009 |

### Phase 3 Tasks (Weeks 7-9)

| Task ID  | Name                 | Effort | Dependencies | Gap(s)  |
| -------- | -------------------- | ------ | ------------ | ------- |
| TASK-015 | Neo4j Storage        | 4d     | Phase 2      | GAP-012 |
| TASK-016 | Qdrant Storage       | 3d     | Phase 2      | GAP-013 |
| TASK-017 | Redis KV             | 3d     | Phase 2      | GAP-024 |
| TASK-018 | Document Scan API    | 2d     | Phase 2      | GAP-014 |
| TASK-019 | Graph Labels Popular | 1d     | Phase 2      | GAP-036 |
| TASK-020 | Reprocess Failed     | 2d     | Phase 2      | GAP-039 |

### Phase 4 Tasks (Weeks 10-12)

| Task ID  | Name                    | Effort | Dependencies | Gap(s) |
| -------- | ----------------------- | ------ | ------------ | ------ |
| TASK-021 | Integration Testing     | 3d     | Phase 3      | All    |
| TASK-022 | Performance Validation  | 2d     | TASK-021     | All    |
| TASK-023 | Documentation & Release | 2d     | TASK-022     | All    |

---

## 🎯 Critical Path

The critical path determines the minimum project duration:

```
TASK-003 (4d) → TASK-006 (1d) → [Phase 1 Gate] →
TASK-010 (3d) → TASK-012 (1d) → TASK-014 (2d) → [Phase 2 Gate] →
TASK-015 (4d) → [Phase 3 Gate] →
TASK-021 (3d) → TASK-022 (2d) → TASK-023 (2d)

Total Critical Path: 22 days (4.5 weeks with parallel work)
```

---

## 📅 Optimal Sequencing

### Week 1

| Day | Developer A      | Developer B         |
| --- | ---------------- | ------------------- |
| 1   | TASK-001 (0.5d)  | TASK-003 (start)    |
| 2   | TASK-002 (start) | TASK-003            |
| 3   | TASK-002         | TASK-003            |
| 4   | TASK-002         | TASK-003            |
| 5   | TASK-004         | TASK-003 (complete) |

### Week 2

| Day | Developer A      | Developer B       |
| --- | ---------------- | ----------------- |
| 1   | TASK-005 (start) | TASK-006          |
| 2   | TASK-005         | Phase 1 Gate prep |
| 3   | Phase 1 Gate     | Phase 1 Gate      |
| 4   | TASK-007 (start) | TASK-010 (start)  |
| 5   | TASK-007         | TASK-010          |

### Week 3-4

Continue Phase 2 tasks in parallel:

- Developer A: TASK-007, TASK-008, TASK-009, TASK-011, TASK-013
- Developer B: TASK-010, TASK-012, TASK-014

### Week 5-7

Phase 3 storage backends (can be parallelized):

- TASK-015, TASK-016, TASK-017 (independent)
- TASK-018, TASK-019, TASK-020 (after storage)

### Week 8-10

Phase 4 validation:

- TASK-021, TASK-022, TASK-023 (sequential)

---

## 🔄 Parallel Execution Opportunities

### Independent Task Groups

**Group A (Phase 1):**

- TASK-001, TASK-002 can start immediately
- TASK-003 can start immediately (independent)

**Group B (Phase 2):**

- TASK-007, TASK-008, TASK-009 can run in parallel
- TASK-010 independent of quality tasks

**Group C (Phase 3):**

- TASK-015, TASK-016, TASK-017 completely independent
- Storage backends can be developed in parallel

---

## ⚠️ Blocking Dependencies

### Hard Dependencies (Must Wait)

| Blocked Task | Waiting For | Reason                     |
| ------------ | ----------- | -------------------------- |
| TASK-004     | TASK-001    | Needs VectorNamespace enum |
| TASK-005     | TASK-002    | Extends Global query       |
| TASK-006     | TASK-003    | Needs TenantRAGManager     |
| TASK-011     | TASK-007    | Uses deduplication         |
| TASK-013     | TASK-011    | Uses summarization output  |

### Soft Dependencies (Can Start Early)

| Task      | Can Start When | Full Completion When   |
| --------- | -------------- | ---------------------- |
| TASK-007  | Phase 1 Gate   | With TASK-002 complete |
| TASK-010  | Phase 1 Gate   | With rate limiter      |
| TASK-015+ | Phase 2 Gate   | Anytime after          |

---

## 📊 Resource Allocation

### Minimum Team Size: 2 developers

| Role  | Phase 1      | Phase 2 | Phase 3 | Phase 4 |
| ----- | ------------ | ------- | ------- | ------- |
| Dev A | Query        | Quality | Storage | Testing |
| Dev B | Multi-tenant | LLM     | API     | Docs    |

### Optimal Team Size: 3 developers

| Role  | Phase 1      | Phase 2   | Phase 3      | Phase 4 |
| ----- | ------------ | --------- | ------------ | ------- |
| Dev A | Query        | Quality   | Neo4j        | Testing |
| Dev B | Multi-tenant | LLM       | Qdrant/Redis | Testing |
| Dev C | Integration  | Reranking | API          | Docs    |

---

## 🔗 Cross-References

| Topic    | Document                                               | Section  |
| -------- | ------------------------------------------------------ | -------- |
| Timeline | [00-INDEX.md](./00-INDEX.md)                           | Roadmap  |
| Risks    | [08-RISK-MITIGATION.md](./08-RISK-MITIGATION.md)       | Blockers |
| Testing  | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md) | Gates    |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake Project Management_
