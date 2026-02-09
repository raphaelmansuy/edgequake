# Tutorial: Query Optimization

> **Choosing and Tuning Query Modes for Best Results**

This tutorial teaches you how to select the right query mode for different question types and optimize retrieval quality.

**Time**: ~20 minutes  
**Level**: Intermediate  
**Prerequisites**: Completed [First RAG App](first-rag-app.md)

---

## Query Mode Overview

EdgeQuake provides 6 query modes, each with different strengths:

```
┌─────────────────────────────────────────────────────────────────┐
│                   QUERY MODE DECISION TREE                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  "What are the main themes?"  ──────────▶  GLOBAL               │
│  (overview, summary)                                            │
│                                                                 │
│  "Who is Sarah Chen?"  ─────────────────▶  LOCAL                │
│  (specific entity)                                              │
│                                                                 │
│  "How does X work?"  ───────────────────▶  HYBRID               │
│  (general questions)                                            │
│                                                                 │
│  "Find documents about..."  ────────────▶  NAIVE                │
│  (keyword search)                                               │
│                                                                 │
│  "Complex multi-part question"  ────────▶  MIX                  │
│  (needs weighted combination)                                   │
│                                                                 │
│  "Just chat, no retrieval"  ────────────▶  BYPASS               │
│  (direct LLM)                                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Mode 1: Naive (Vector Only)

**Best for**: Simple keyword lookups, document similarity

### How It Works

```
Query ──▶ [Embed] ──▶ [Vector Search] ──▶ Top K Chunks ──▶ LLM ──▶ Answer
```

### Example

```bash
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "funding announcement",
    "mode": "naive"
  }'
```

### When to Use

| ✅ Good For           | ❌ Avoid For            |
| --------------------- | ---------------------- |
| Keyword search        | Multi-hop reasoning    |
| Finding similar docs  | Relationship questions |
| Simple factual lookup | Overview questions     |
| Fast responses        | Complex analysis       |

### Tuning Parameters

```json
{
  "query": "funding announcement",
  "mode": "naive",
  "max_chunks": 10,
  "similarity_threshold": 0.7
}
```

---

## Mode 2: Local (Entity-Focused)

**Best for**: Questions about specific entities and their relationships

### How It Works

```
Query ──▶ [Extract Entities] ──▶ [Graph Traversal] ──▶ Related Context ──▶ LLM ──▶ Answer
                                        │
                                        ▼
                              Entity descriptions
                              Related entities
                              Relationships
                              Source chunks
```

### Example

```bash
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is Sarah Chen'\''s background and role?",
    "mode": "local"
  }'
```

### When to Use

| ✅ Good For          | ❌ Avoid For         |
| -------------------- | ------------------- |
| "Who is X?"          | Overview questions  |
| "What does X do?"    | Theme analysis      |
| Entity relationships | When entity unknown |
| Biography questions  | General how-tos     |

### Tuning Parameters

```json
{
  "query": "Sarah Chen's background",
  "mode": "local",
  "max_entities": 10,
  "max_hops": 2,
  "include_relationships": true
}
```

---

## Mode 3: Global (Community Summaries)

**Best for**: Overview questions, theme analysis, corpus-wide insights

### How It Works

```
Query ──▶ [Match Communities] ──▶ [Community Summaries] ──▶ LLM ──▶ Answer
                                         │
                                         ▼
                               Pre-computed summaries
                               of entity clusters
```

### Example

```bash
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What are the main themes and topics across all documents?",
    "mode": "global"
  }'
```

### When to Use

| ✅ Good For      | ❌ Avoid For          |
| ---------------- | --------------------- |
| "Main themes?"   | Specific entity facts |
| "Overview of..." | Detailed how-tos      |
| "Key topics?"    | Finding specific docs |
| Summary requests | Precise citations     |

### Tuning Parameters

```json
{
  "query": "main themes",
  "mode": "global",
  "max_communities": 5,
  "community_level": 0
}
```

---

## Mode 4: Hybrid (Default - Combined)

**Best for**: General questions, balanced context needs

### How It Works

```
                              ┌──▶ [Vector Search] ────┐
Query ──▶ [Parallel] ─────────┼──▶ [Entity Lookup] ────┼──▶ [Combine] ──▶ LLM ──▶ Answer
                              └──▶ [Community Match] ──┘
```

### Example

```bash
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How has TechCorp evolved since its founding?",
    "mode": "hybrid"
  }'
```

### When to Use

| ✅ Good For         | ❌ Avoid For           |
| ------------------- | ---------------------- |
| General questions   | When speed is critical |
| Unsure of best mode | Simple keyword search  |
| Default choice      | Specific edge cases    |
| Complex questions   |                        |

### Tuning Parameters

```json
{
  "query": "TechCorp evolution",
  "mode": "hybrid",
  "max_chunks": 10,
  "max_entities": 10,
  "max_communities": 3
}
```

---

## Mode 5: Mix (Weighted Combination)

**Best for**: Fine-tuned blending of retrieval strategies

### How It Works

```
                              ┌──▶ [Vector] ─────▶ Score × 0.4 ─┐
Query ──▶ [Parallel] ─────────┤                                  ├──▶ [Rank] ──▶ LLM
                              └──▶ [Entity] ─────▶ Score × 0.6 ─┘
```

### Example

```bash
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "NeuralSearch capabilities and key people",
    "mode": "mix",
    "vector_weight": 0.3,
    "entity_weight": 0.5,
    "community_weight": 0.2
  }'
```

### When to Use

| ✅ Good For            | ❌ Avoid For           |
| ---------------------- | ---------------------- |
| Custom optimization    | Quick queries          |
| A/B testing modes      | When unsure of weights |
| Domain-specific tuning | General use            |
| Production fine-tuning |                        |

### Weight Presets

| Use Case       | Vector | Entity | Community |
| -------------- | ------ | ------ | --------- |
| Factual lookup | 0.7    | 0.2    | 0.1       |
| Relationship Q | 0.2    | 0.7    | 0.1       |
| Overview Q     | 0.1    | 0.2    | 0.7       |
| Balanced       | 0.4    | 0.4    | 0.2       |

---

## Mode 6: Bypass (Direct LLM)

**Best for**: When retrieval isn't needed

### How It Works

```
Query ──▶ [Direct LLM Call] ──▶ Answer
           (no retrieval)
```

### Example

```bash
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=$WORKSPACE_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is the capital of France?",
    "mode": "bypass"
  }'
```

### When to Use

| ✅ Good For       | ❌ Avoid For       |
| ----------------- | ------------------ |
| General knowledge | Document questions |
| Code generation   | Anything in corpus |
| Format conversion | Fact-checking      |
| Math/logic        | Citations needed   |

---

## Choosing the Right Mode

### Decision Flowchart

```
                           Question Type?
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
    About specific        General/mixed       Overview/themes
       entity?               question?            wanted?
          │                    │                    │
          ▼                    ▼                    ▼
        LOCAL               HYBRID               GLOBAL
          │                    │                    │
          │                    │                    │
     Need more?           Need tuning?        Need more?
          │                    │                    │
          ▼                    ▼                    ▼
        HYBRID                MIX                HYBRID
```

### Quick Reference

| Question Pattern         | Best Mode     |
| ------------------------ | ------------- |
| "Who is X?"              | local         |
| "What is X?"             | hybrid        |
| "How does X work?"       | hybrid        |
| "Main themes?"           | global        |
| "Overview of..."         | global        |
| "Find docs about..."     | naive         |
| "Compare X and Y"        | hybrid or mix |
| "X's relationship to Y?" | local         |

---

## Performance Comparison

### Latency by Mode

| Mode   | Avg Latency | Notes                |
| ------ | ----------- | -------------------- |
| naive  | ~200ms      | Fastest, vector only |
| local  | ~300ms      | Graph traversal      |
| global | ~400ms      | Community matching   |
| hybrid | ~500ms      | Parallel, combined   |
| mix    | ~500ms      | Like hybrid          |
| bypass | ~100ms      | No retrieval         |

### Quality by Question Type

| Question Type | Naive    | Local    | Global   | Hybrid   |
| ------------- | -------- | -------- | -------- | -------- |
| Entity facts  | ⭐⭐     | ⭐⭐⭐⭐ | ⭐⭐     | ⭐⭐⭐   |
| Relationships | ⭐       | ⭐⭐⭐⭐ | ⭐⭐     | ⭐⭐⭐   |
| Overview      | ⭐       | ⭐⭐     | ⭐⭐⭐⭐ | ⭐⭐⭐   |
| Similarity    | ⭐⭐⭐⭐ | ⭐⭐     | ⭐       | ⭐⭐⭐   |
| Complex       | ⭐       | ⭐⭐⭐   | ⭐⭐⭐   | ⭐⭐⭐⭐ |

---

## Advanced Tuning

### Context Window Management

Control how much context goes to the LLM:

```json
{
  "query": "Detailed analysis of TechCorp",
  "mode": "hybrid",
  "max_context_tokens": 8000,
  "response_max_tokens": 2000
}
```

### Similarity Thresholds

Filter out low-quality matches:

```json
{
  "query": "specific technical term",
  "mode": "naive",
  "similarity_threshold": 0.8
}
```

Higher threshold = fewer but more relevant results.

### Temperature Control

Adjust LLM creativity:

```json
{
  "query": "Summarize the findings",
  "mode": "global",
  "temperature": 0.3
}
```

| Temperature | Behavior                |
| ----------- | ----------------------- |
| 0.0 - 0.3   | Factual, deterministic  |
| 0.4 - 0.7   | Balanced (default: 0.7) |
| 0.8 - 1.0   | Creative, varied        |

---

## A/B Testing Modes

Compare modes programmatically:

```python
import requests

WORKSPACE_ID = "ws_abc123"
QUERY = "What are TechCorp's main products and leadership?"

modes = ["naive", "local", "global", "hybrid"]
results = {}

for mode in modes:
    resp = requests.post(
        f"http://localhost:8080/api/v1/query?workspace_id={WORKSPACE_ID}",
        json={"query": QUERY, "mode": mode}
    )
    result = resp.json()
    results[mode] = {
        "answer": result["answer"][:200],
        "sources": len(result.get("sources", [])),
        "entities": len(result.get("entities_used", [])),
        "latency": result.get("latency_ms", 0)
    }

# Compare results
for mode, data in results.items():
    print(f"\n=== {mode.upper()} ===")
    print(f"Answer: {data['answer']}...")
    print(f"Sources: {data['sources']}, Entities: {data['entities']}")
    print(f"Latency: {data['latency']}ms")
```

---

## Common Issues

### Too Few Results

**Symptoms**: Empty or very short answers.

**Solutions**:

1. Lower `similarity_threshold`
2. Increase `max_chunks` or `max_entities`
3. Try `hybrid` mode instead of `naive`

### Irrelevant Results

**Symptoms**: Answer doesn't match question.

**Solutions**:

1. Increase `similarity_threshold`
2. Use more specific mode (`local` for entity questions)
3. Check if documents cover the topic

### Slow Queries

**Symptoms**: Latency > 2 seconds.

**Solutions**:

1. Reduce `max_context_tokens`
2. Use `naive` mode for simple questions
3. Check LLM provider latency

---

## What You Learned

✅ All 6 query modes and their strengths  
✅ How to choose the right mode for each question  
✅ Tuning parameters for optimization  
✅ Performance characteristics  
✅ A/B testing approaches  
✅ Common issues and solutions

---

## Next Steps

| Tutorial                                  | Description                 |
| ----------------------------------------- | --------------------------- |
| [Multi-Tenant Setup](multi-tenant.md)     | Building a SaaS application |
| [Custom Entity Types](custom-entities.md) | Domain-specific extraction  |
| [API Integration](api-integration.md)     | Building on EdgeQuake       |

---

## See Also

- [Query Modes Deep-Dive](../deep-dives/query-modes.md) - Detailed algorithm explanation
- [REST API](../api-reference/rest-api.md) - Query endpoint reference
- [Hybrid Retrieval](../concepts/hybrid-retrieval.md) - Conceptual overview
