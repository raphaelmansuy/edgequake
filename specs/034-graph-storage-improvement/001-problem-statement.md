# SPEC-034-001: Problem Statement & Observed Symptoms

> **Lens**: Product / User Experience  
> **Version**: 1.0.0 — 2026-06-30

---

## 1. The Complaint

> *"Storing a large document into the Knowledge Graph takes a long time."*

A user uploads a 300-page PDF (academic paper, contract, investor deck). The
system extracts **hundreds to thousands of entities and relationships**. The
status transitions from `Uploading → Extracting → Storing`.  
**The `Storing` phase can take minutes or never complete.**

---

## 2. Observed Symptoms

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ SYMPTOM MAP                                                                 │
│                                                                             │
│  Document Size ──────┐                                                      │
│  (pages / tokens)    │                                                      │
│                      ▼                                                      │
│  LLM Extraction    [100–2000 entities extracted]                            │
│                      │                                                      │
│                      ▼                                                      │
│  Graph Storing  ──► [STATUS: Storing ... ⏳ 30s ... 60s ... 120s ...]      │
│                      │                                                      │
│                      ├── Pipeline status: "Storing in knowledge graph..."  │
│                      ├── UI shows spinning progress badge                  │
│                      └── Sometimes: "Pipeline processing failed: Entity    │
│                              extraction e..."  (timeout / OOM)              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Measured Processing Times (Production)

| Document                | Entities          | Approx store time                |
| ----------------------- | ----------------- | -------------------------------- |
| Small PDF (5 pages)     | ~20 entities      | < 2 seconds                      |
| Medium PDF (50 pages)   | ~200 entities     | 15–45 seconds                    |
| Large PDF (200+ pages)  | ~500–800 entities | 2–8 minutes                      |
| Academic paper (jepa)   | 2,179 entities    | 8–15 minutes                     |
| Investor deck (ADF III) | ~500 entities     | Still storing at time of writing |

### 2.2 Scaling Behaviour

The problem is **super-linear**: doubling the entity count more than doubles
the store time. This is the hallmark of an **O(N × M)** algorithm operating on
a growing collection, not an O(N) batch write.

```
Time (s)
  │
  120 │                                       *
  │                                      /
   90 │                                  /
  │                              /
   60 │                       /
  │                   /
   30 │            /
  │       /
    5 │  /
  └────────────────────────────────────────── Entities
     50    200    500   1000   2000
```

---

## 3. Stakeholder Impact

| Stakeholder        | Pain                                                                   |
| ------------------ | ---------------------------------------------------------------------- |
| End user           | Cannot use freshly-uploaded documents for minutes                      |
| System operator    | Backend appears frozen; hard to distinguish from failure               |
| AI engineer        | Entity knowledge base grows stale while documents queue                |
| DevOps             | CPU spikes during storage phase; no horizontal scaling helps           |
| LightRAG algorithm | Deduplication quality degrades when writes race against ongoing merges |

---

## 4. Business Constraint

**The fix must preserve automatic database migrations.**  

- Zero downtime deploys are required
- Migration scripts are applied automatically via sqlx `_sqlx_migrations`
- Any structural change must be backward-compatible with existing data
- Indexes must be created `CONCURRENTLY` to avoid locking production tables

---

## 5. Non-Goals (for this spec)

- Improving LLM entity extraction latency (that is a separate concern)
- Changing the data model (graph nodes/edges structure stays the same)
- Adding new API endpoints or user-facing features
- Replacing Apache AGE with a different graph engine
