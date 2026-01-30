# OODA Iteration 06 - Observe

**Date**: 2025-01-XX
**Focus**: Query Modes Deep-Dive Documentation

## 🔍 Observations

### 1. Query Modes Code Analysis

**File**: `edgequake-query/src/modes.rs` (~150 lines)

EdgeQuake implements 5 query modes (LightRAG has only 3):

| Mode   | FEAT     | Vector | Graph | Use Case                  |
| ------ | -------- | ------ | ----- | ------------------------- |
| Naive  | FEAT0101 | ✅     | ❌    | Fast factual queries      |
| Local  | FEAT0102 | ✅     | ✅    | Entity relationships      |
| Global | FEAT0103 | ❌     | ✅    | Theme/topic queries       |
| Hybrid | FEAT0104 | ✅     | ✅    | Complex queries (default) |
| Mix    | FEAT0105 | ✅     | ✅    | Custom weighted blend     |

Note: Bypass mode (FEAT0106) is mentioned in docs but not in enum - may be handled elsewhere.

### 2. Engine Implementation

**File**: `edgequake-query/src/engine.rs` (~690 lines)

Key methods:

- `query()` - Main entry point, orchestrates retrieval
- `retrieve_context()` - Mode-aware context building
- `uses_vector_search()` / `uses_graph()` - Mode capability detection

Default config:

- `max_chunks: 10`
- `max_entities: 20`
- `max_context_tokens: 4000`
- `graph_depth: 2`
- `min_score: 0.1`

### 3. Performance Trade-offs

From the code documentation:

```
Mode    | Speed | Accuracy | Context Size
--------|-------|----------|-------------
Naive   | Fast  | Good     | Small (chunks only)
Local   | Med   | High     | Medium (entity + neighbors)
Global  | Slow  | High     | Large (community summaries)
Hybrid  | Slow  | Best     | Large (both approaches)
```

### 4. Missing Documentation

- No deep-dive exists for query modes
- Algorithm selection logic needs explanation
- Use case examples with actual queries needed
- Performance benchmarks would be valuable
