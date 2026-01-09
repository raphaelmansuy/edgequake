# Iteration 65 - ORIENT: Strategy Refinement

**Date:** 2026-01-09
**Input:** observe.md findings (45 duplicates, 110 undocumented, 32 namespace violations)

## Analysis

### The Root Cause Cascade

```
┌─────────────────────────────────────────────────────────────┐
│                    HISTORICAL FAILURE                        │
├─────────────────────────────────────────────────────────────┤
│  1. No namespace allocation defined                          │
│     ↓                                                        │
│  2. Developers picked FEAT IDs arbitrarily                   │
│     ↓                                                        │
│  3. Same IDs used for different features (collision)         │
│     ↓                                                        │
│  4. Same IDs used for feature aspects (overloading)          │
│     ↓                                                        │
│  5. Frontend used backend IDs for frontend features          │
│     ↓                                                        │
│  6. Documentation didn't keep up                             │
│     ↓                                                        │
│  7. 62.1% documentation gap, 45 duplicates                   │
└─────────────────────────────────────────────────────────────┘
```

### Pattern Classification

The 45 duplicates fall into 3 categories:

#### Category A: Intentional Overloading (Acceptable)

Features that represent the same conceptual capability across layers:

- FEAT0001: Document ingestion (types + stores + components + API)
- FEAT0007: Query processing (types + stores + components + API)
- FEAT0202: Graph operations (stores + components)

**Strategy:** Keep single ID, document as "cross-cutting feature"

#### Category B: Accidental Collision (Critical)

Different features accidentally share an ID:

- FEAT0701: Lineage visualization vs API client core
- FEAT0702: Entity tracing vs Request interceptors
- FEAT0801: Cost tracking (Frontend) vs Auth (Backend)

**Strategy:** Reassign one feature to new ID

#### Category C: Namespace Violation (Recoverable)

Frontend-specific features using backend namespace:

- FEAT0501-0506: Auth/Tenant features using LLM namespace
- FEAT0801-0804: Cost features using Auth namespace

**Strategy:** Migrate to correct frontend namespace (06XX, 07XX, 085X, 086X)

### Revised Namespace Allocation

| Range    | Module                   | Team     | Status                            |
| -------- | ------------------------ | -------- | --------------------------------- |
| FEAT00XX | Core Engine              | Backend  | Keep                              |
| FEAT01XX | Query Engine             | Backend  | Keep                              |
| FEAT02XX | Graph Operations         | Backend  | Keep (frontend references OK)     |
| FEAT03XX | Streaming                | Backend  | Keep                              |
| FEAT04XX | PDF Processing           | Backend  | Keep                              |
| FEAT05XX | LLM Integration          | Backend  | **Clear: Remove Auth/Tenant**     |
| FEAT06XX | WebUI Core               | Frontend | Expand usage                      |
| FEAT07XX | API Client               | Frontend | **Clear: Remove Lineage overlap** |
| FEAT08XX | Authentication (Backend) | Backend  | Stays 0800-0849                   |
| FEAT085X | Cost Management          | Frontend | **New: 0850-0859**                |
| FEAT086X | WebUI Providers          | Frontend | **New: 0860-0869**                |
| FEAT087X | Auth UI                  | Frontend | **New: 0870-0879**                |
| FEAT10XX | Document Management      | Frontend | Keep                              |

### Migration Plan

#### Phase 1: Critical Collisions (5 IDs)

| Old ID   | Feature A (Keep)   | Feature B (Migrate)  | New ID   |
| -------- | ------------------ | -------------------- | -------- |
| FEAT0701 | Chunk lineage      | SSE streaming client | FEAT0750 |
| FEAT0702 | Entity tracing     | Request interceptors | FEAT0751 |
| FEAT0703 | Lineage views      | Chat completions API | FEAT0752 |
| FEAT0704 | Chunk filtering    | Streaming chat       | FEAT0753 |
| FEAT0705 | Entity exploration | Query mode           | FEAT0754 |

#### Phase 2: Namespace Corrections (12 IDs)

| Old ID   | Feature            | Wrong NS | Correct NS | New ID   |
| -------- | ------------------ | -------- | ---------- | -------- |
| FEAT0501 | Auth/JWT tokens    | LLM      | Auth UI    | FEAT0870 |
| FEAT0504 | Tenant context     | LLM      | Providers  | FEAT0861 |
| FEAT0505 | Session management | LLM      | Auth UI    | FEAT0871 |
| FEAT0506 | Tenant switching   | LLM      | Providers  | FEAT0862 |
| FEAT0801 | Per-doc cost       | Auth     | Cost       | FEAT0850 |
| FEAT0803 | Workspace cost     | Auth     | Cost       | FEAT0851 |
| FEAT0804 | Cost breakdown     | Auth     | Cost       | FEAT0852 |

#### Phase 3: Documentation Update (110 features)

- Generate entries from code using generate_registry.py
- Mark overloaded features as "cross-cutting"
- Update features.md index

## Risk Assessment

| Risk                                      | Probability | Impact | Mitigation                                 |
| ----------------------------------------- | ----------- | ------ | ------------------------------------------ |
| Missing some @implements during migration | Medium      | Low    | Use grep to verify                         |
| Breaking existing tracking dashboards     | Low         | Medium | Grep for FEAT ID usage first               |
| Re-introducing duplicates                 | Low         | High   | Run validate_features.py after each change |

## Recommendation for DECIDE Phase

**Strategy: Incremental Migration with Validation**

1. Fix Category B collisions first (highest priority, 5 IDs)
2. Fix Category C namespace violations second (12 IDs)
3. Update documentation third (110 features)
4. Accept Category A overloading (document as cross-cutting)
5. Add CI/CD validation to prevent recurrence

**Time Estimate:** 3-4 iterations (66-69)
