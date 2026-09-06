# Lens — Embedding & Graph Expert (SPEC-146)

## Outcome

Typed ANN pre-filters by document allow-set; graph expansion is provenance-gated; hubs remain unlabeled; community reports do not mix classifications.

## ANN — target

```ascii
  TODAY (typed production)
    chunk_embeddings WHERE workspace_id
    document_ids in MetadataFilter IGNORED

  TARGET
    chunk_embeddings ce
    JOIN chunks c ON c.id = ce.chunk_id
    WHERE ce.workspace_id = $ws
      AND c.document_id = ANY($allow)
    ORDER BY embedding <=> $q

  Entity / relationship fleet
    constrain via source_ids ∩ allow
    OR run ANN then drop seeds with zero authorized provenance
    (prefer SQL constraint where possible)
```

Post-filter (`filter_context_by_document_ids`) remains a **safety net**, never the only control.

## Graph hops

```ascii
  Seed entities (authorized provenance)
         │
         v
  expand_neighborhood_edges / BFS / PPR
         │
         ├─ KEEP edge if source_ids ∩ allow ≠ ∅
         └─ DROP otherwise (fail-closed; invert "any doc" merge)

  Hub description @ query
         │
         └─ concatenate authorized occurrence texts only
```

## Ban label-union

```ascii
  Doc A Secret ──┐
                 ├──► Entity X hub   ✗ store max(classification) on hub
  Doc B Public ──┘
                 └──► OK: per-occurrence labels via source_ids
```

## Community / global

```ascii
  Mixed-label workspace + community reports ON
       │
       ├─ Option 1: disable community arm when any classified docs exist
       ├─ Option 2: build reports per authz partition (costly)
       └─ v1 default recommendation: skip community inject under ABAC
          unless reports are single-classification
```

## Popular nodes / degree

`GET /graph/labels/popular` and degree batch must compute on **authorized** topology only (or return empty when ABAC on and unscoped).

## Delete / tombstone

```ascii
  Document delete
       │
       ├─ chunks CASCADE
       ├─ chunk_embeddings gone
       ├─ AGE source_ids pruned (SPEC-098 cascade rules)
       └─ vector tombstone / serving fence — no residual ANN hit (EC-146-13)
```

## Perf notes

| Topic | Stance |
|-------|--------|
| JOIN vs denorm `document_id` | JOIN first; denorm if p95 regresses |
| Underfill when \|A\|/\|W\| small | iterative_scan; measure recall UNCONFIRMED |
| Materialized per-principal graphs | Out of v1 |

## Cross-refs

- Code as-is → [../03-code-as-is.md](../03-code-as-is.md)  
- Data model → [../05-data-model.md](../05-data-model.md)  
- AI lens → [009-ai-engineer.md](009-ai-engineer.md)  
- EC-146-04..07,13,24  
