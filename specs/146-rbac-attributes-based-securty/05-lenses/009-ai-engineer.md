# Lens — AI Engineer (SPEC-146)

## Outcome

GraphRAG modes remain useful on the **authorized** neighborhood. Entity hubs stay shared ontology; secrets stay on occurrences. LLM never becomes the PDP.

## Prompt / context rules

```ascii
  Context assembly
       │
       ├─ chunks: allow-set only
       ├─ entities: descriptions from authorized occurrences only
       ├─ relationships: provenance-gated
       ├─ community: partitioned or omitted
       └─ citations: authorized titles only

  ✗ No "hidden docs exist" system hints
  ✗ No denied ids in tool schemas / SSE
  ✗ Prompt injection cannot elevate allow-set
```

## Mode impacts

| Mode | Change |
|------|--------|
| Naive | Chunk ANN pre-filter |
| Local | Entity ANN ∩ provenance; hops gated |
| Global | Rel ANN + community partition/skip |
| Hybrid/Mix | Each arm gated; merge cannot reintroduce denied |

## Quality tradeoffs (honest)

```ascii
  Stricter entity descriptions (authorized-only)
       │
       v
  Possible recall drop on multi-doc entities
       │
       v
  Acc impact = UNCONFIRMED (F-146-32)
  Measure on medical-mid / SPEC-001 harness AFTER M3
  Do NOT claim Acc win/loss in marketing
```

## Caching

SPEC-103 caches must include principal + `policy_version` or be disabled when ABAC on. Keyword cache especially dangerous (no context in key today).

## Extraction write path

At extract time: **do not** denormalize multi-doc secrets onto hub `description`. Store occurrence-level text; assemble at query (LAW-146-10).

## MCP / agents

Treat agents as `query_agent` principals. No `bypass_acl`. Retrieval ids bound to minting principal.

## Cross-refs

- Architecture → [../04-target-architecture.md](../04-target-architecture.md)  
- Embedding/graph → [010-embedding-graph.md](010-embedding-graph.md)  
- Threat model → [../11-threat-model.md](../11-threat-model.md)  
